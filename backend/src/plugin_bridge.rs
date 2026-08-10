//! PyO3 Bridge - Native Rust ↔ Python integration
//! Replaces fragile spawn-based bridge with embedded Python interpreter

use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList};
use serde_json::Value;

#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct PluginResult {
    pub success: bool,
    pub data: Option<Value>,
    pub error: Option<String>,
}

/// Initialize Python interpreter (called once at startup)
pub fn init_python() -> Result<(), String> {
    Python::with_gil(|py| {
        // Add le-coq paths to sys.path
        let sys = py
            .import("sys")
            .map_err(|e| format!("Failed to import sys: {}", e))?;
        let path = sys
            .getattr("path")
            .map_err(|e| format!("Failed to get sys.path: {}", e))?;
        let path_list = path
            .downcast::<PyList>()
            .map_err(|e| format!("Failed to downcast path: {}", e))?;
        
        // Add paths in order: plugins site-packages first (for deps), then plugins, then le-coq root
        // INSERT en tête (position 0) : un VIRTUAL_ENV externe peut injecter
        // un venv Python 3.14 en premier, masquant les deps cp311.
        path_list
            .insert(0, "/home/yaya/le-coq/plugins/.venv/lib/python3.11/site-packages")
            .map_err(|e| format!("Failed to insert site-packages: {}", e))?;
        path_list
            .append("/home/yaya/le-coq/plugins")
            .map_err(|e| format!("Failed to append plugins: {}", e))?;
        path_list
            .append("/home/yaya/le-coq")
            .map_err(|e| format!("Failed to append le-coq root: {}", e))?;

        // debug: print sys.path + probe import
        let path_str: String = path.str().map_err(|e| e.to_string())?.to_string();
        eprintln!("[plugin_bridge] sys.path = {}", path_str);
        match py.import("pydantic_core") {
            Ok(m) => eprintln!("[plugin_bridge] pydantic_core OK: {:?}", m.getattr("__file__")),
            Err(e) => eprintln!("[plugin_bridge] pydantic_core FAIL: {}", e),
        }

        Ok(())
    })
}

/// Call a Python plugin method with arguments
pub fn call_plugin_method(
    plugin_name: &str,
    method: &str,
    args: &Value,
) -> Result<PluginResult, String> {
    Python::with_gil(|py| {
        // Import the plugins module
        let plugins_module = py
            .import("plugins")
            .map_err(|e| format!("Failed to import plugins module: {}", e))?;

        // Call the sync wrapper method
        let args_dict = json_to_pydict(py, args)
            .map_err(|e| e.to_string())?;
        let result_obj = plugins_module
            .getattr("call_plugin_method_sync")
            .map_err(|e| format!("Failed to get call_plugin_method_sync: {}", e))?
            .call1((plugin_name, method, args_dict))
            .map_err(|e| {
                format!("Failed to call method '{}.{}': {}", plugin_name, method, e)
            })?;

        // Convert result to JSON
        let result_value = pydict_to_json(result_obj)
            .map_err(|e| format!("Failed to convert Python result to JSON: {}", e))?;

        Ok(PluginResult {
            success: true,
            data: Some(result_value),
            error: None,
        })
    })
}

/// List all available plugins
pub fn list_plugins() -> Result<Vec<String>, String> {
    Python::with_gil(|py| {
        // Import the plugins registry
        let plugins_module = py
            .import("plugins")
            .map_err(|e| format!("Failed to import plugins module: {}", e))?;

        // Call list_plugins()
        let result = plugins_module
            .call_method0("list_plugins")
            .map_err(|e| format!("Failed to call list_plugins: {}", e))?;

        // Convert to Vec<String>
        let list = result
            .downcast::<PyList>()
            .map_err(|e| format!("list_plugins did not return a list: {}", e))?;

        let mut plugins = Vec::new();
        for item in list.iter() {
            if let Ok(s) = item.extract::<String>() {
                plugins.push(s);
            }
        }

        Ok(plugins)
    })
}

/// Get plugins that failed to import, with their error messages
pub fn plugin_failures() -> Result<Vec<(String, String)>, String> {
    Python::with_gil(|py| {
        let plugins_module = py
            .import("plugins")
            .map_err(|e| format!("Failed to import plugins module: {}", e))?;

        let result = plugins_module
            .call_method0("plugin_failures")
            .map_err(|e| format!("Failed to call plugin_failures: {}", e))?;

        let dict = result
            .downcast::<PyDict>()
            .map_err(|e| format!("plugin_failures did not return a dict: {}", e))?;

        let mut failures = Vec::new();
        for (k, v) in dict.iter() {
            let name: String = k.extract().map_err(|e| e.to_string())?;
            let err: String = v.extract().map_err(|e| e.to_string())?;
            failures.push((name, err));
        }
        Ok(failures)
    })
}

/// Reload all plugins from disk (hot-reload after editing a plugin file)
pub fn reload_plugins() -> Result<Vec<String>, String> {
    Python::with_gil(|py| {
        // Re-import the plugins module itself (picks up __init__ changes),
        // then reload each source submodule.
        let importlib = py
            .import("importlib")
            .map_err(|e| format!("Failed to import importlib: {}", e))?;
        importlib
            .call_method0("invalidate_caches")
            .map_err(|e| format!("Failed to invalidate caches: {}", e))?;

        let plugins_module = py
            .import("plugins")
            .map_err(|e| format!("Failed to import plugins module: {}", e))?;
        let plugins_module = importlib
            .call_method1("reload", (plugins_module,))
            .map_err(|e| format!("Failed to reload plugins module: {}", e))?;

        // Reload every plugins.sources.* submodule
        let sources_pkg = py
            .import("plugins.sources")
            .map_err(|e| format!("Failed to import plugins.sources: {}", e))?;
        let sources_pkg = importlib
            .call_method1("reload", (sources_pkg,))
            .map_err(|e| format!("Failed to reload plugins.sources: {}", e))?;
        let _ = sources_pkg; // keep binding to avoid GC before submodule reloads

        let sys = py.import("sys").map_err(|e| e.to_string())?;
        let modules = sys.getattr("modules").map_err(|e| e.to_string())?;
        let modules = modules
            .downcast::<PyDict>()
            .map_err(|e| e.to_string())?;
        let mut to_reload = Vec::new();
        for (k, v) in modules.iter() {
            if let Ok(name) = k.extract::<String>() {
                if name.starts_with("plugins.sources.") {
                    to_reload.push(v);
                }
            }
        }
        for m in to_reload {
            let _ = importlib.call_method1("reload", (m,)); // ignore per-module errors
        }

        // Re-run discovery to register new plugins / record failures
        let discover = plugins_module
            .getattr("_auto_discover")
            .map_err(|e| format!("Failed to get _auto_discover: {}", e))?;
        discover
            .call0()
            .map_err(|e| format!("Failed to re-run auto-discovery: {}", e))?;

        list_plugins()
    })
}

/// Validate a plugin connection
pub fn validate_plugin(plugin_name: &str) -> Result<bool, String> {
    Python::with_gil(|py| {
        // Import the plugins module
        let plugins_module = py
            .import("plugins")
            .map_err(|e| format!("Failed to import plugins module: {}", e))?;

        // Call the sync wrapper method
        let result_obj = plugins_module
            .getattr("call_plugin_method_sync")
            .map_err(|e| format!("Failed to get call_plugin_method_sync: {}", e))?
            .call1((plugin_name, "validate", PyDict::new(py)))
            .map_err(|e| {
                format!("Failed to call validate on '{}': {}", plugin_name, e)
            })?;

        result_obj
            .extract::<bool>()
            .map_err(|e| format!("validate() did not return a bool: {}", e))
    })
}

/// Helper: Convert serde_json::Value to PyDict
fn json_to_pydict(py: Python, value: &Value) -> PyResult<PyObject> {
    match value {
        Value::Object(map) => {
            let dict = PyDict::new(py);
            for (k, v) in map {
                dict.set_item(k, json_to_pydict(py, v)?)?;
            }
            Ok(dict.into())
        }
        Value::Array(arr) => {
            let list = PyList::empty(py);
            for item in arr {
                list.append(json_to_pydict(py, item)?)?;
            }
            Ok(list.into())
        }
        Value::String(s) => Ok(s.into_py(py)),
        Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                Ok(i.into_py(py))
            } else if let Some(f) = n.as_f64() {
                Ok(f.into_py(py))
            } else {
                Ok(0.into_py(py))
            }
        }
        Value::Bool(b) => Ok(b.into_py(py)),
        Value::Null => Ok(py.None()),
    }
}

/// Helper: Convert Python object to serde_json::Value
fn pydict_to_json(obj: &PyAny) -> PyResult<Value> {
    if obj.is_none() {
        return Ok(Value::Null);
    }

    if let Ok(b) = obj.extract::<bool>() {
        return Ok(Value::Bool(b));
    }

    if let Ok(i) = obj.extract::<i64>() {
        return Ok(Value::Number(i.into()));
    }

    if let Ok(f) = obj.extract::<f64>() {
        return Ok(serde_json::Number::from_f64(f)
            .map(Value::Number)
            .unwrap_or(Value::Null));
    }

    if let Ok(s) = obj.extract::<String>() {
        return Ok(Value::String(s));
    }

    if let Ok(list) = obj.downcast::<PyList>() {
        let mut arr = Vec::new();
        for item in list.iter() {
            arr.push(pydict_to_json(item)?);
        }
        return Ok(Value::Array(arr));
    }

    if let Ok(dict) = obj.downcast::<PyDict>() {
        let mut map = serde_json::Map::new();
        for (key, value) in dict.iter() {
            let k = key.extract::<String>()?;
            let v = pydict_to_json(value)?;
            map.insert(k, v);
        }
        return Ok(Value::Object(map));
    }

    // Fallback: try to convert to string
    if let Ok(s) = obj.str() {
        return Ok(Value::String(s.to_string_lossy().into()));
    }

    Ok(Value::Null)
}

/// Capitalize first letter of a string
fn capitalize_first(s: &str) -> String {
    if s.is_empty() {
        return String::new();
    }
    let mut chars = s.chars();
    chars.next().unwrap().to_uppercase().collect::<String>() + chars.as_str()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_capitalize_first() {
        assert_eq!(capitalize_first("wikipedia"), "Wikipedia");
        assert_eq!(capitalize_first("acled"), "Acled");
        assert_eq!(capitalize_first("gdelt"), "Gdelt");
        assert_eq!(capitalize_first("opencorporates"), "Opencorporates");
        assert_eq!(capitalize_first(""), "");
    }

    #[test]
    fn test_json_to_pydict_string() {
        Python::with_gil(|py| {
            let value = Value::String("hello".to_string());
            let result = json_to_pydict(py, &value).unwrap();
            let s = result.extract::<String>(py).unwrap();
            assert_eq!(s, "hello");
        });
    }

    #[test]
    fn test_json_to_pydict_number() {
        Python::with_gil(|py| {
            let value = Value::Number(serde_json::Number::from(42));
            let result = json_to_pydict(py, &value).unwrap();
            let n = result.extract::<i64>(py).unwrap();
            assert_eq!(n, 42);
        });
    }

    #[test]
    fn test_json_to_pydict_bool() {
        Python::with_gil(|py| {
            let value = Value::Bool(true);
            let result = json_to_pydict(py, &value).unwrap();
            let b = result.extract::<bool>(py).unwrap();
            assert_eq!(b, true);
        });
    }

    #[test]
    fn test_json_to_pydict_null() {
        Python::with_gil(|py| {
            let value = Value::Null;
            let result = json_to_pydict(py, &value).unwrap();
            assert!(result.is_none(py));
        });
    }

    #[test]
    fn test_json_to_pydict_array() {
        Python::with_gil(|py| {
            let value = Value::Array(vec![
                Value::Number(serde_json::Number::from(1)),
                Value::Number(serde_json::Number::from(2)),
                Value::Number(serde_json::Number::from(3)),
            ]);
            let result = json_to_pydict(py, &value).unwrap();
            let list = result.downcast::<PyList>(py).unwrap();
            assert_eq!(list.len(), 3);
            assert_eq!(list.get_item(0).unwrap().extract::<i64>().unwrap(), 1);
        });
    }

    #[test]
    fn test_json_to_pydict_object() {
        Python::with_gil(|py| {
            let mut map = serde_json::Map::new();
            map.insert("key".to_string(), Value::String("value".to_string()));
            map.insert("count".to_string(), Value::Number(serde_json::Number::from(10)));
            let value = Value::Object(map);
            let result = json_to_pydict(py, &value).unwrap();
            let dict = result.downcast::<PyDict>(py).unwrap();
            assert_eq!(
                dict.get_item("key")
                    .unwrap()
                    .unwrap()
                    .extract::<String>()
                    .unwrap(),
                "value"
            );
            assert_eq!(
                dict.get_item("count")
                    .unwrap()
                    .unwrap()
                    .extract::<i64>()
                    .unwrap(),
                10
            );
        });
    }

    #[test]
    fn test_pydict_to_json_string() {
        Python::with_gil(|py| {
            let obj: PyObject = "hello".into_py(py);
            let result = pydict_to_json(obj.as_ref(py)).unwrap();
            assert_eq!(result, Value::String("hello".to_string()));
        });
    }

    #[test]
    fn test_pydict_to_json_number() {
        Python::with_gil(|py| {
            let obj = 42i64.into_py(py);
            let result = pydict_to_json(obj.as_ref(py)).unwrap();
            assert_eq!(result, Value::Number(serde_json::Number::from(42)));
        });
    }

    #[test]
    fn test_pydict_to_json_bool() {
        Python::with_gil(|py| {
            let obj = true.into_py(py);
            let result = pydict_to_json(obj.as_ref(py)).unwrap();
            assert_eq!(result, Value::Bool(true));
        });
    }

    #[test]
    fn test_pydict_to_json_null() {
        Python::with_gil(|py| {
            let obj = py.None();
            let result = pydict_to_json(obj.as_ref(py)).unwrap();
            assert_eq!(result, Value::Null);
        });
    }

    #[test]
    fn test_pydict_to_json_list() {
        Python::with_gil(|py| {
            let list = PyList::new(py, &[1i64, 2i64, 3i64]);
            let result = pydict_to_json(list.as_ref()).unwrap();
            assert_eq!(result, Value::Array(vec![
                Value::Number(serde_json::Number::from(1)),
                Value::Number(serde_json::Number::from(2)),
                Value::Number(serde_json::Number::from(3)),
            ]));
        });
    }

    #[test]
    fn test_pydict_to_json_dict() {
        Python::with_gil(|py| {
            let dict = PyDict::new(py);
            dict.set_item("name", "test").unwrap();
            dict.set_item("value", 123i64).unwrap();
            let result = pydict_to_json(dict.as_ref()).unwrap();
            let obj = result.as_object().unwrap();
            assert_eq!(obj.get("name").unwrap(), &Value::String("test".to_string()));
            assert_eq!(obj.get("value").unwrap(), &Value::Number(serde_json::Number::from(123)));
        });
    }

    #[test]
    fn test_list_plugins() {
        // Initialize Python first
        init_python().expect("Failed to init Python");

        let plugins = list_plugins().expect("Failed to list plugins");
        assert!(!plugins.is_empty(), "Should have at least one plugin");

        // Check known plugins exist (wikipedia, gdelt, opencorporates)
        assert!(plugins.contains(&"wikipedia".to_string()));
        assert!(plugins.contains(&"gdelt".to_string()));
        assert!(plugins.contains(&"opencorporates".to_string()));
    }

    #[test]
    fn test_validate_plugin_wikipedia() {
        init_python().expect("Failed to init Python");
        
        let valid = validate_plugin("wikipedia").expect("Failed to validate plugin");
        // Wikipedia plugin should be valid (no API key required)
        assert!(valid, "Wikipedia plugin should be valid");
    }

    #[test]
    fn test_call_plugin_method_wikipedia_fetch() {
        init_python().expect("Failed to init Python");
        
        let args = serde_json::json!({
            "query": "Rust programming language",
            "limit": 1
        });
        
        let result = call_plugin_method("wikipedia", "fetch", &args)
            .expect("Failed to call plugin method");
        
        assert!(result.success, "Plugin call should succeed");
        assert!(result.data.is_some(), "Should return data");
        
        let data = result.data.unwrap();
        // Fetch returns a list of results
        assert!(data.is_array() || data.is_object(), "Data should be array or object");
    }
}
