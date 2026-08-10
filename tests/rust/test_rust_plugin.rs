use std::process::Command;

fn main() {
    // Test calling the Python plugin system
    let output = Command::new("venv/bin/python")
        .arg("-c")
        .arg("import sys; sys.path.insert(0, 'plugins'); from plugins.sources.wikipedia import WikipediaPlugin; plugin = WikipediaPlugin(); result = plugin.search('test'); print(result)")
        .output()
        .expect("Failed to execute Python command");

    if output.status.success() {
        let stdout = String::from_utf8_lossy(&output.stdout);
        println!("Python plugin output: {}", stdout);
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        println!("Python plugin error: {}", stderr);
    }
}