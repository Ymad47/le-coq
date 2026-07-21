use std::process::Command;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
struct PluginResult {
    success: bool,
    data: Option<serde_json::Value>,
    error: Option<String>,
}

fn call_python_plugin(source: &str, query: &serde_json::Value) -> Result<PluginResult, String> {
    // Create a temporary script to call the plugin
    let script = format!(
        r#"
import sys
import json
import asyncio
sys.path.insert(0, 'plugins')

async def main():
    try:
        # Import the specific plugin to register it
        if '{}' == 'wikipedia':
            import plugins.sources.wikipedia
        
        # Import the plugin system
        from plugins import get_plugin
        plugin = get_plugin('{}')
        
        if plugin is None:
            raise Exception('Unknown plugin: {}')
        
        result = await plugin.fetch({})
        
        # Return the result as JSON
        print(json.dumps({{
            'success': True,
            'data': result
        }}))
    except Exception as e:
        print(json.dumps({{
            'success': False,
            'error': str(e)
        }}))

asyncio.run(main())
"#,
        source, source, source, query
    );

    let output = Command::new("venv/bin/python")
        .current_dir("..")
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

fn main() {
    let query = serde_json::json!({"query": "test"});
    match call_python_plugin("wikipedia", &query) {
        Ok(result) => {
            println!("Success: {}", result.success);
            if let Some(data) = result.data {
                println!("Data: {}", data);
            }
            if let Some(error) = result.error {
                println!("Error: {}", error);
            }
        }
        Err(e) => {
            println!("Error: {}", e);
        }
    }
}