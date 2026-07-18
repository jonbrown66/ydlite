use std::ffi::OsStr;

use tokio::process::Command;

#[cfg(target_os = "windows")]
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

pub fn hidden_command(program: impl AsRef<OsStr>) -> Command {
    let mut command = Command::new(program);
    #[cfg(target_os = "windows")]
    command.creation_flags(CREATE_NO_WINDOW);
    command
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn creates_a_command_without_changing_the_program() {
        let command = hidden_command("ffmpeg");
        assert_eq!(command.as_std().get_program(), "ffmpeg");
    }
}
