/// Installe OSMOzzz au démarrage automatique.
/// - macOS  : LaunchAgent (~/.plist)
/// - Windows : dossier Startup
/// - Linux  : systemd user service
use anyhow::{Context, Result};

pub fn run() -> Result<()> {
    #[cfg(target_os = "macos")]
    return install_macos();

    #[cfg(target_os = "windows")]
    return install_windows();

    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    return install_linux();
}

// ─── macOS — LaunchAgent ──────────────────────────────────────────────────────

#[cfg(target_os = "macos")]
fn install_macos() -> Result<()> {
    const PLIST_LABEL: &str = "com.osmozzz.daemon";

    let home = dirs_next::home_dir().context("Home introuvable")?;
    let launch_agents = home.join("Library/LaunchAgents");
    std::fs::create_dir_all(&launch_agents)?;

    let exe = std::env::current_exe().context("Exe introuvable")?;
    let exe_path = exe.to_str().context("Chemin exe invalide")?;
    let plist_path = launch_agents.join(format!("{}.plist", PLIST_LABEL));

    let plist = format!(r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>Label</key>
    <string>{label}</string>
    <key>ProgramArguments</key>
    <array>
        <string>{exe}</string>
        <string>daemon</string>
    </array>
    <key>RunAtLoad</key>
    <true/>
    <key>KeepAlive</key>
    <true/>
    <key>StandardOutPath</key>
    <string>{log}</string>
    <key>StandardErrorPath</key>
    <string>{log}</string>
    <key>EnvironmentVariables</key>
    <dict>
        <key>ORT_DYLIB_PATH</key>
        <string>/opt/homebrew/lib/libonnxruntime.dylib</string>
    </dict>
</dict>
</plist>
"#,
        label = PLIST_LABEL,
        exe = exe_path,
        log = home.join(".osmozzz/daemon.log").display(),
    );

    std::fs::write(&plist_path, &plist)?;

    let status = std::process::Command::new("launchctl")
        .args(["load", "-w", plist_path.to_str().unwrap()])
        .status();

    match status {
        Ok(s) if s.success() => {
            println!("✓ OSMOzzz installé et démarré.");
            println!("  Dashboard : http://localhost:7878");
            println!("  Démarre automatiquement à chaque login.");
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

// ─── Windows — dossier Startup ────────────────────────────────────────────────

#[cfg(target_os = "windows")]
fn install_windows() -> Result<()> {
    let exe = std::env::current_exe().context("Exe introuvable")?;

    // Crée un raccourci dans %APPDATA%\Microsoft\Windows\Start Menu\Programs\Startup
    let startup = dirs_next::data_dir()
        .context("AppData introuvable")?
        .join("Microsoft/Windows/Start Menu/Programs/Startup");
    std::fs::create_dir_all(&startup)?;

    // Copie le binaire ou crée un .bat qui lance "osmozzz daemon"
    let bat = startup.join("osmozzz-daemon.bat");
    let bat_content = format!(
        "@echo off\nstart /b \"\" \"{}\" daemon\n",
        exe.display()
    );
    std::fs::write(&bat, bat_content)?;

    println!("✓ OSMOzzz installé dans le dossier Startup Windows.");
    println!("  Dashboard : http://localhost:7878");
    println!("  Démarre automatiquement à chaque login.");
    println!("  Pour désinstaller : osmozzz uninstall");
    Ok(())
}

// ─── Linux — message d'aide ───────────────────────────────────────────────────

#[cfg(not(any(target_os = "macos", target_os = "windows")))]
fn install_linux() -> Result<()> {
    let exe = std::env::current_exe().context("Exe introuvable")?;
    println!("Pour installer OSMOzzz au démarrage sur Linux, créez un service systemd user :");
    println!();
    println!("  mkdir -p ~/.config/systemd/user/");
    println!("  cat > ~/.config/systemd/user/osmozzz.service << EOF");
    println!("  [Unit]");
    println!("  Description=OSMOzzz daemon");
    println!();
    println!("  [Service]");
    println!("  ExecStart={} daemon", exe.display());
    println!("  Restart=always");
    println!();
    println!("  [Install]");
    println!("  WantedBy=default.target");
    println!("  EOF");
    println!();
    println!("  systemctl --user enable --now osmozzz");
    Ok(())
}
