use notify_rust::Notification;

pub fn spawn_background_process() -> std::io::Result<()> {
    let args: Vec<String> = std::env::args().filter(|a| a != "--background").collect();

    let child = std::process::Command::new(&args[0])
        .args(&args[1..])
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()?;

    println!("Download started in background (PID: {})", child.id());
    Ok(())
}

pub fn notify_send(msg: String) {
    if std::env::var("DISPLAY").is_ok() {
        let _ = Notification::new().summary("dwrs").body(&msg).show();
    } else {
        println!("{}", msg);
    }
}
