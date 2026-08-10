use std::process::Command;

fn main() {
    // Test calling the Python plugin system
    let script = r#"
import sys
import json
import asyncio
sys.path.insert(0, 'plugins')

async def main():
    try:
        # Import the specific plugin to register it
        import plugins.sources.wikipedia
        
        # Import the plugin system
        from plugins import get_plugin
        plugin = get_plugin('wikipedia')
        
        if plugin is None:
            raise Exception('Unknown plugin: wikipedia')
        
        result = await plugin.fetch({'query': 'test'})
        
        # Return the result as JSON
        print(json.dumps({
            'success': True,
            'data': result
        }))
    except Exception as e:
        print(json.dumps({
            'success': False,
            'error': str(e)
        }))

asyncio.run(main())
"#;

    let output = Command::new("venv/bin/python")
        .arg("-c")
        .arg(script)
        .output()
        .expect("Failed to execute Python command");

    if !output.status.success() {
        panic!(
            "Python command failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    println!("Output: {}", stdout);
}