use std::env;
use std::process::ExitCode;

const HELP: &str = "\
Entrybound (experimental Rust bootstrap)\n\
\n\
Usage: entrybound <COMMAND>\n\
\n\
The native archive operations are not implemented in the language-migration\n\
slice. The command boundary is established for: pack, unpack, list, inspect,\n\
and verify.\n";

fn run(arguments: impl IntoIterator<Item = String>) -> ExitCode {
    let mut arguments = arguments.into_iter();
    let _program = arguments.next();
    match arguments.next().as_deref() {
        None | Some("help" | "--help" | "-h") => {
            print!("{HELP}");
            ExitCode::SUCCESS
        }
        Some(command) => {
            eprintln!(
                "UNSUPPORTED EB_CLI_NOT_IMPLEMENTED: command '{command}' is not implemented yet"
            );
            ExitCode::from(2)
        }
    }
}

fn main() -> ExitCode {
    run(env::args())
}

#[cfg(test)]
mod tests {
    use super::run;
    use std::process::ExitCode;

    #[test]
    fn help_is_available_without_claiming_commands_work() {
        assert_eq!(run(["entrybound".to_owned()]), ExitCode::SUCCESS);
    }

    #[test]
    fn unfinished_commands_fail_explicitly() {
        assert_eq!(
            run(["entrybound".to_owned(), "pack".to_owned()]),
            ExitCode::from(2)
        );
    }
}
