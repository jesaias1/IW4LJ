pub fn startup_commands() -> Vec<String> {
    startup_commands_from(std::env::args(), std::env::var("IW4L_CMDS").ok())
}

pub fn strip_cmds_flag(args: impl Iterator<Item = String>) -> Vec<String> {
    let mut skip_value = false;
    args.filter(|arg| {
        let drop = skip_value || arg == "--cmds" || arg.starts_with("--cmds=");
        skip_value = arg == "--cmds";
        !drop
    })
    .collect()
}

fn startup_commands_from(args: impl Iterator<Item = String>, env: Option<String>) -> Vec<String> {
    let mut scripts = Vec::new();
    let mut args = args.skip_while(|a| a != "--cmds" && !a.starts_with("--cmds="));
    match args.next() {
        Some(flag) if flag == "--cmds" => scripts.extend(args.next()),
        Some(inline) => scripts.push(inline["--cmds=".len()..].to_owned()),
        None => {}
    }
    scripts.extend(env);
    scripts
        .iter()
        .flat_map(|script| script.split(';'))
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(str::to_owned)
        .collect()
}

#[derive(bevy::prelude::Resource)]
pub(crate) struct StdinCommands(std::sync::Mutex<std::sync::mpsc::Receiver<String>>);

pub(crate) fn install_stdin(app: &mut bevy::prelude::App) {
    if std::env::var("IW4L_CONSOLE_STDIN").as_deref() != Ok("1") {
        return;
    }
    let (sender, receiver) = std::sync::mpsc::sync_channel(256);
    std::thread::spawn(move || {
        use std::io::BufRead;
        for line in std::io::stdin().lock().lines() {
            let Ok(line) = line else { break };
            if sender.send(line).is_err() {
                break;
            }
        }
    });
    app.insert_resource(StdinCommands(std::sync::Mutex::new(receiver)));
}

pub(crate) fn drain_stdin(
    input: Option<bevy::prelude::Res<StdinCommands>>,
    mut queue: bevy::prelude::ResMut<crate::ConsoleQueue>,
) {
    let Some(input) = input else { return };
    let Ok(receiver) = input.0.lock() else { return };
    for line in receiver.try_iter().take(256) {
        queue.push_line(&line);
    }
}
