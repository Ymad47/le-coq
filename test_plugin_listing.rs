use std::process::Command;

fn main() {
    // Test listing Python plugins
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