use std::process::Command;
use serde_json::Value;

#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct PluginResult {
    pub success: bool,
    pub data: Option<Value>,
    pub error: Option<String>,
}

pub fn call_python_plugin(source: &str, query: &Value) -> Result<PluginResult, String> {
    // Create a temporary script to call the plugin
    let script = format!(
        r#"
import sys
import json
import asyncio
sys.path.insert(0, '/home/yaya/le-coq')

async def main():
    try:
        # Import the specific plugin to register it
        if '{}' == 'wikipedia':
            from plugins.sources.wikipedia import WikipediaPlugin
            plugin = WikipediaPlugin({{}})
        else:
            raise Exception('Unknown plugin: {}')
        
        result = await plugin.fetch({})
        
        # Serialize the result properly
        serialized_result = []
        for item in result:
            # Convert any datetime objects to strings
            if hasattr(item, 'created_at') and item.created_at:
                item.created_at = item.created_at.isoformat()
            serialized_result.append(item)
        
        # Return the result as JSON
        print(json.dumps({{
            'success': True,
            'data': serialized_result
        }}, default=str))
    except Exception as e:
        print(json.dumps({{
            'success': False,
            'error': str(e)
        }}))

asyncio.run(main())
"#,
        source, source, query
    );

    let output = Command::new("/home/yaya/le-coq/venv/bin/python")
        .arg("-c")
        .arg(script)
        .output()
        .map_err(|e| format!("Failed to execute Python command: {}", e))?;

    if !output.status.success() {
        return Err(format!(
            "Python command failed with exit code {}: {}",
            output.status,
            String::from_utf8_lossy(&output.stderr)
        ));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    
    // Parse the JSON response
    let result: PluginResult = serde_json::from_str(&stdout)
        .map_err(|e| format!("Failed to parse Python response: {}", e))?;

    Ok(result)
}

pub fn list_python_plugins() -> Result<Vec<String>, String> {
    let script = r#"
import sys
import json
sys.path.insert(0, '/home/yaya/le-coq')

try:
    # Import plugins to register them
    from plugins.sources.wikipedia import WikipediaPlugin
    
    # Create an instance and register it
    plugin = WikipediaPlugin({})
    
    from plugins import list_plugins
    plugins = list_plugins()
    print(json.dumps(list(plugins)))
except Exception as e:
    print(json.dumps({'error': str(e)}))
"#;

    let output = Command::new("/home/yaya/le-coq/venv/bin/python")
        .arg("-c")
        .arg(script)
        .output()
        .map_err(|e| format!("Failed to execute Python command: {}", e))?;

    if !output.status.success() {
        return Err(format!(
            "Python command failed: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    
    // Try to parse as JSON array
    if let Ok(plugins) = serde_json::from_str::<Vec<String>>(&stdout) {
        return Ok(plugins);
    }
    
    // Try to parse as JSON object with error
    if let Ok(error_obj) = serde_json::from_str::<serde_json::Value>(&stdout) {
        if let Some(error) = error_obj.get("error") {
            return Err(error.as_str().unwrap_or("Unknown error").to_string());
        }
    }
    
    // Fallback: split by lines
    let plugins: Vec<String> = stdout
        .lines()
        .map(|line| line.trim().to_string())
        .filter(|line| !line.is_empty())
        .collect();

    Ok(plugins)
}