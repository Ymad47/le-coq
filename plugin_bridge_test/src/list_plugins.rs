use std::process::Command;
use serde::{Deserialize, Serialize};

fn list_python_plugins() -> Result<Vec<String>, String> {
    // Create a temporary script to list plugins
    let script = r#"
import sys
import json
sys.path.insert(0, 'plugins')

try:
    # Import plugins to register them
    import plugins.sources.wikipedia
    
    from plugins import list_plugins
    plugins = list_plugins()
    print(json.dumps(list(plugins)))
except Exception as e:
    print(json.dumps({'error': str(e)}))
"#;

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
    let plugins: Vec<String> = serde_json::from_str(&stdout)
        .map_err(|e| format!("Failed to parse Python response: {}", e))?;

    Ok(plugins)
}

fn main() {
    match list_python_plugins() {
        Ok(plugins) => {
            println!("Available plugins:");
            for plugin in plugins {
                println!("  - {}", plugin);
            }
        }
        Err(e) => {
            println!("Error: {}", e);
        }
    }
}