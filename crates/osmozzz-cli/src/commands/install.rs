/// Installe OSMOzzz comme LaunchAgent macOS.
/// Lance le daemon automatiquement à chaque login.
use anyhow::{Context, Result};

const PLIST_LABEL: &str = "com.osmozzz.daemon";

pub fn run() -> Result<()> {
    let home = dirs_next::home_dir().context("Cannot find home directory")?;
    let launch_agents = home.join("Library/LaunchAgents");
    std::fs::create_dir_all(&launch_agents)?;

    let exe = std::env::current_exe().context("Cannot find current executable")?;
    let exe_path = exe.to_str().context("Invalid exe path")?;

    let plist_path = launch_agents.join(format!("{}.plist", PLIST_LABEL));

    let plist = format!(r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>Label</key>
    <string>{}</string>
    <key>ProgramArguments</key>
    <array>
        <string>{}</string>
        <string>daemon</string>
    </array>
    <key>RunAtLoad</key>
    <true/>
    <key>KeepAlive</key>
    <true/>
    <key>StandardOutPath</key>
    <string>{}</string>
    <key>StandardErrorPath</key>
    <string>{}</string>
    <key>EnvironmentVariables</key>
    <dict>
        <key>ORT_DYLIB_PATH</key>
        <string>/opt/homebrew/lib/libonnxruntime.dylib</string>
    </dict>
</dict>
</plist>
"#,
        PLIST_LABEL,
        exe_path,
        home.join(".osmozzz/daemon.log").display(),
        home.join(".osmozzz/daemon.log").display(),
    );

    std::fs::write(&plist_path, &plist)?;

    // Charger immédiatement
    let status = std::process::Command::new("launchctl")
        .args(["load", "-w", plist_path.to_str().unwrap()])
        .status();

    match status {
        Ok(s) if s.success() => {
            println!("✓ OSMOzzz installé et démarré.");
            println!("  Dashboard : http://localhost:7878");
            println!("  Il démarrera automatiquement à chaque login.");
            println!("");
            println!("  Pour désinstaller : osmozzz uninstall");
        }
        _ => {
            println!("✓ Plist installé : {}", plist_path.display());
            println!("  Le daemon démarrera au prochain login.");
            println!("  Pour démarrer maintenant : osmozzz daemon");
        }
    }

    Ok(())
}
