// maybe consider rewriting with https://github.com/TimonPost/crossterm

use termion;
use termion::color;
use termion::event::Key;
use termion::input::TermRead;
use termion::raw::IntoRawMode;

use rustyline::error::ReadlineError;
use rustyline::Editor;

use std::io::{stdin, stdout, BufRead, Write};
use std::process::Command;

use select::{draw_select, Entry};
use version_control_actions::VersionControlActions;

const RESET_COLOR: color::Fg<color::Reset> = color::Fg(color::Reset);
const RESET_BG_COLOR: color::Bg<color::Reset> = color::Bg(color::Reset);

const HEADER_COLOR: color::Fg<color::Rgb> = color::Fg(color::Rgb(0, 0, 0));
const HEADER_BG_COLOR: color::Bg<color::Rgb> = color::Bg(color::Rgb(255, 0, 255));
const ACTION_COLOR: color::Fg<color::Rgb> = color::Fg(color::Rgb(255, 100, 180));
const ENTRY_COLOR: color::Fg<color::Rgb> = color::Fg(color::Rgb(255, 180, 100));

const DONE_COLOR: color::Fg<color::LightGreen> = color::Fg(color::LightGreen);
const CANCEL_COLOR: color::Fg<color::LightYellow> = color::Fg(color::LightYellow);
const ERROR_COLOR: color::Fg<color::Red> = color::Fg(color::Red);

const VERSION: &'static str = env!("CARGO_PKG_VERSION");

pub fn show_tui<'a, T: VersionControlActions>(repository_name: &str, version_control: &'a mut T) {
	let _guard = termion::init();

	let stdin = stdin();
	let stdin = stdin.lock();
	let stdout = stdout().into_raw_mode().unwrap();

	Tui::new(stdin, stdout, repository_name, version_control).show();
}

struct Tui<'a, R: BufRead, W: Write, T: VersionControlActions + 'a> {
	stdin: R,
	stdout: W,
	repository_name: &'a str,
	version_control: &'a mut T,
	readline: Editor<()>,
}

impl<'a, R: BufRead, W: Write, T: VersionControlActions> Tui<'a, R, W, T> {
	fn new(stdin: R, stdout: W, repository_name: &'a str, version_control: &'a mut T) -> Self {
		Tui {
			stdin: stdin,
			stdout: stdout,
			repository_name: repository_name,
			version_control: version_control,
			readline: Editor::new(),
		}
	}

	fn show(&mut self) {
		self.show_header();
		self.show_help();

		loop {
			if let Some(Ok(key)) = (&mut self.stdin).keys().next() {
				match key {
					Key::Ctrl('c') => break,
					Key::Ctrl(key) => self.handle_key(key, true),
					Key::Char(key) => self.handle_key(key, false),
					_ => (),
				}
			}

			self.stdout.flush().unwrap();
		}
	}

	fn handle_key(&mut self, key: char, is_control_held: bool) {
		if is_control_held {
			match key {
				'b' => {
					self.show_action("close branch");
					if let Some(input) = self.handle_input("branch to close (ctrl+c to cancel): ") {
						let result = self.version_control.close_branch(&input[..]);
						self.handle_result(result);
					}
				}
				'r' => {
					self.show_action("merge taking local");
					let result = self.version_control.take_local();
					self.handle_result(result);
				}
				_ => (),
			}
		} else {
			match key {
				'h' => {
					self.show_action("help");
					self.show_help();
				}
				'e' => {
					self.show_action("explorer");
					self.open_explorer();
				}
				's' => {
					self.show_action("status");
					let result = self.version_control.status();
					self.handle_result(result);
				}
				'l' => {
					self.show_action("log");
					let result = self.version_control.log();
					self.handle_result(result);
				}
				'd' => {
					self.show_action("revision changes");
					if let Some(input) = self.handle_input("show changes from (ctrl+c to cancel): ")
					{
						let result = self.version_control.changes(&input[..]);
						self.handle_result(result);
					}
				}
				'D' => {
					self.show_action("revision diff");
					if let Some(input) = self.handle_input("show diff from (ctrl+c to cancel): ") {
						let result = self.version_control.diff(&input[..]);
						self.handle_result(result);
					}
				}
				'c' => {
					self.show_action("commit all");

					if let Some(input) = self.handle_input("commit message (ctrl+c to cancel): ") {
						let result = self.version_control.commit_all(&input[..]);
						self.handle_result(result);
					}
				}
				'C' => {
					self.show_action("commit selected");

					match self.version_control.get_files_to_commit() {
						Ok(mut entries) => {
							self.show_add_remove_ui(&mut entries);
							write!(self.stdout, "\n\n").unwrap();

							if let Some(input) =
								self.handle_input("commit message (ctrl+c to cancel): ")
							{
								let result =
									self.version_control.commit_selected(&input[..], &entries);
								self.handle_result(result);
							}
						}
						Err(error) => self.handle_result(Err(error)),
					}
				}
				'U' => {
					self.show_action("revert");
					let result = self.version_control.revert();
					self.handle_result(result);
				}
				'u' => {
					self.show_action("update");
					if let Some(input) = self.handle_input("update to (ctrl+c to cancel): ") {
						let result = self.version_control.update(&input[..]);
						self.handle_result(result);
					}
				}
				'm' => {
					self.show_action("merge");
					if let Some(input) = self.handle_input("merge with (ctrl+c to cancel): ") {
						let result = self.version_control.merge(&input[..]);
						self.handle_result(result);
					}
				}
				'r' => {
					self.show_action("unresolved conflicts");
					let result = self.version_control.conflicts();
					self.handle_result(result);
				}
				'R' => {
					self.show_action("merge taking other");
					let result = self.version_control.take_other();
					self.handle_result(result);
				}
				'f' => {
					self.show_action("fetch");
					let result = self.version_control.fetch();
					self.handle_result(result);
				}
				'p' => {
					self.show_action("pull");
					let result = self.version_control.pull();
					self.handle_result(result);
				}
				'P' => {
					self.show_action("push");
					let result = self.version_control.push();
					self.handle_result(result);
				}
				'T' => {
					self.show_action("tag");
					if let Some(input) = self.handle_input("tag name (ctrl+c to cancel): ") {
						let result = self.version_control.create_tag(&input[..]);
						self.handle_result(result);
					}
				}
				'b' => {
					self.show_action("branches");
					let result = self.version_control.list_branches();
					self.handle_result(result);
				}
				'B' => {
					self.show_action("branch");
					if let Some(input) = self.handle_input("branch name (ctrl+c to cancel): ") {
						let result = self.version_control.create_branch(&input[..]);
						self.handle_result(result);
					}
				}
				_ => (),
			}
		}
	}

	fn handle_input(&mut self, prompt: &str) -> Option<String> {
		write!(self.stdout, "{}{}{}\n", ENTRY_COLOR, prompt, RESET_COLOR).unwrap();

		let readline = self
			.readline
			//.readline(&format!("{}{}{}", ENTRY_COLOR, prompt, RESET_COLOR)[..]);
			.readline("");

		match readline {
			Ok(line) => Some(line),
			Err(ReadlineError::Interrupted) | Err(ReadlineError::Eof) => {
				write!(
					self.stdout,
					"\n\n{}canceled{}\n\n",
					CANCEL_COLOR, RESET_COLOR
				)
				.unwrap();
				None
			}
			Err(err) => {
				println!("error {:?}\n\n", err);
				None
			}
		}
	}

	fn handle_result(&mut self, result: Result<String, String>) {
		match result {
			Ok(output) => {
				write!(self.stdout, "{}\n\n", output).unwrap();
				write!(self.stdout, "{}done{}\n\n", DONE_COLOR, RESET_COLOR).unwrap();
			}
			Err(error) => {
				write!(self.stdout, "{}\n\n", error).unwrap();
				write!(self.stdout, "{}error{}\n\n", ERROR_COLOR, RESET_COLOR).unwrap();
			}
		}
	}

	fn show_header(&mut self) {
		write!(self.stdout, "{}", termion::clear::All).unwrap();

		if let Ok((w, _)) = termion::terminal_size() {
			write!(
				self.stdout,
				"{}{}{}",
				termion::cursor::Goto(1, 1),
				HEADER_COLOR,
				HEADER_BG_COLOR,
			)
			.unwrap();

			write!(self.stdout, "{}", " ".repeat(w as usize)).unwrap();
		}

		write!(
			self.stdout,
			"{}{}Verco @ {}{}{}\n\n",
			HEADER_COLOR,
			termion::cursor::Goto(1, 1),
			self.repository_name,
			RESET_COLOR,
			RESET_BG_COLOR,
		)
		.unwrap();

		self.stdout.flush().unwrap();
	}

	fn show_action(&mut self, action_name: &str) {
		self.show_header();
		write!(
			self.stdout,
			"{}{}{}\n\n",
			ACTION_COLOR, action_name, RESET_COLOR
		)
		.unwrap();
	}

	fn show_help(&mut self) {
		write!(self.stdout, "Verco {}\n\n", VERSION).unwrap();

		match self.version_control.version() {
			Ok(version) => {
				write!(self.stdout, "{}", version).unwrap();
				write!(self.stdout, "\n\n").unwrap();
			}
			Err(error) => {
				write!(self.stdout, "{}{}", ERROR_COLOR, error).unwrap();
				panic!("Could not find version control in system");
			}
		}

		write!(self.stdout, "press a key and peform an action\n\n").unwrap();

		self.show_help_action("h", "help\n");

		self.show_help_action("e", "explorer\n");

		self.show_help_action("s", "status");
		self.show_help_action("l", "log\n");

		self.show_help_action("d", "revision changes");
		self.show_help_action("shift+d", "revision diff\n");

		self.show_help_action("c", "commit all");
		self.show_help_action("shift+c", "commit selected");
		self.show_help_action("shift+u", "revert");
		self.show_help_action("u", "update/checkout");
		self.show_help_action("m", "merge\n");

		self.show_help_action("r", "unresolved conflicts");
		self.show_help_action("shift+r", "resolve taking other");
		self.show_help_action("ctrl+r", "resolve taking local\n");

		self.show_help_action("f", "fetch");
		self.show_help_action("p", "pull");
		self.show_help_action("shift+p", "push\n");

		self.show_help_action("shift+t", "create tag\n");

		self.show_help_action("b", "list branches");
		self.show_help_action("shift+b", "create branch");
		self.show_help_action("ctrl+b", "close branch\n");
	}

	fn show_help_action(&mut self, shortcut: &str, action: &str) {
		write!(
			self.stdout,
			"\t{}{}{}\t\t{}\n",
			ENTRY_COLOR, shortcut, RESET_COLOR, action
		)
		.unwrap();
	}

	fn open_explorer(&mut self) {
		let mut command = Command::new("explorer");
		command.arg(self.repository_name);
		command.spawn().expect("failed to open explorer");

		write!(self.stdout, "{}done{}\n\n", DONE_COLOR, RESET_COLOR).unwrap();
	}

	pub fn show_add_remove_ui(&mut self, entries: &mut Vec<Entry>) {
		let mut index = 0;

		loop {
			write!(self.stdout, "{}", termion::clear::All).unwrap();
			self.show_action("commit selected");

			if !draw_select(&mut self.stdin, &mut self.stdout, entries, &mut index) {
				break;
			}
		}
	}
}