use tokio::process::Command;
use tokio::io::{BufReader, AsyncBufReadExt, AsyncWriteExt};
use std::process::Stdio;
use std::sync::mpsc::{Sender, Receiver};
use std::sync::mpsc;
use std::collections::HashMap;

/// position
#[derive(Debug)]
pub enum Position {
	/// set up position from fen
	Fen { fen: String },
	/// set up position from fen and uci move list as string
	FenAndMovesStr { fen: String, moves_str: String },
	/// set up position from startpos
	Startpos,
	/// set up position from startpos and uci move list as string
	StartposAndMovesStr { moves_str: String },
}

use Position::*;

/// uci engine
#[derive(Debug)]
pub struct UciEngine {
	/// command path, example `./stockfish`
	path: String,
	/// handle to process stdin, used internally
	stdin: tokio::process::ChildStdin,
	/// receiver for bestmove, used internally
	rx: Receiver<String>,
}

/// go command job
#[derive(Debug)]
pub struct GoJob {
	/// uci options as key value pairs
	uci_options: HashMap<String, String>,
	/// position setup
	position: Position,
	/// go command options as key value pairs
	go_options: HashMap<String, String>,
}

/// time control
#[derive(Debug)]
pub struct Timecontrol {
	/// white time
	wtime: usize,
	/// white increment
	winc: usize,
	/// black time
	btime: usize,
	/// black increment
	binc: usize,
}

/// implementation of time control
impl Timecontrol {
	/// create default time control
	/// one minute thinking time for both sides, no increment
	pub fn default() -> Timecontrol {
		Timecontrol {
			wtime: 60000,
			winc: 0,
			btime: 60000,
			binc: 0,
		}
	}
}

/// go command job implementation
impl GoJob {
	/// create new GoJob with reasonable defaults
	pub fn new() -> GoJob {
		GoJob {
			position: Startpos,
			uci_options: HashMap::new(),
			go_options: HashMap::new(),
		}
	}
	
	/// set position and return self
	pub fn pos(mut self, pos: Position) -> GoJob {
		self.position = pos;
		
		self
	}
	
	/// set uci option as key value pair and return self
	pub fn uci_opt(mut self, key:String, value:String) -> GoJob {
		self.uci_options.insert(key, value);
		
		self
	}
	
	/// set go option as key value pair and return self
	pub fn go_opt(mut self, key:String, value:String) -> GoJob {
		self.go_options.insert(key, value);
		
		self
	}
	
	/// set time control and return self
	pub fn tc(mut self, tc: Timecontrol) -> GoJob {
		self.go_options.insert("wtime".to_string(), format!("{}", tc.wtime));
		self.go_options.insert("winc".to_string(),  format!("{}", tc.winc));
		self.go_options.insert("btime".to_string(), format!("{}", tc.btime));
		self.go_options.insert("binc".to_string(),  format!("{}", tc.binc));
		
		self
	}
}

/// go command result
#[derive(Debug)]
pub struct GoResult {
	/// best move if any
	bestmove: Option<String>,
	/// ponder if any
	ponder: Option<String>,
}

/// uci engine implementation
impl UciEngine {
	/// create new uci engine and spawn it
	/// path should hold command path, example `./stockfish12`
	pub fn new(path: String) -> UciEngine {		
		let mut cmd = Command::new(path.as_str());
		
		cmd.stdout(Stdio::piped());
		cmd.stdin(Stdio::piped());
	
		let mut child = cmd.spawn()
        	.expect("failed to spawn command");
		
		let stdout = child.stdout.take()
        	.expect("child did not have a handle to stdout");
	
		let stdin = child.stdin.take()
			.expect("child did not have a handle to stdin");
		
		let reader = BufReader::new(stdout).lines();
		
		let (tx, rx):(Sender<String>, Receiver<String>) = mpsc::channel();

		tokio::spawn(async {
			let status = child.await
				.expect("child process encountered an error");

			println!("child status was: {}", status);
		});

		tokio::spawn(async {
			match UciEngine::read_stdout(tx, reader).await {
				Ok(result) => println!("reader ok {:?}", result),
				Err(err) => println!("reader err {:?}", err)
			}
		});

		println!("spawned uci engine : {}", path);
		
		UciEngine {
			path: path,
			stdin: stdin,
			rx: rx,
		}
	}
	
	/// read engine stdout, used internally
	async fn read_stdout(
		tx: Sender<String>,
		mut reader: tokio::io::Lines<tokio::io::BufReader<tokio::process::ChildStdout>>
	) -> Result<(), Box<dyn std::error::Error>> {
		while let Some(line) = reader.next_line().await? {
			println!("engine out : {}", line);
			if line.len() >= 8 {
				if &line[0..8] == "bestmove" {
					let _ = tx.send(line);					
				}	
			}
		}

		Ok(())
	}

	/// issue uci command, used internally
	async fn issue_command(&mut self, command: String) -> Result<(), Box<dyn std::error::Error>> {
		println!("issuing uci command : {}", command);
		
		let _ = self.stdin.write_all(format!("{}\n", command).as_bytes()).await?;

		Ok(())
	}
	
	/// start thinking based on go job and return result, blocking
	pub async fn go(&mut self, go_job: GoJob) -> Result<GoResult, Box<dyn std::error::Error>> {
		for (key, value) in go_job.uci_options {
			self.issue_command(format!("setoption name {} value {}", key, value).to_string()).await?;
		}
		
		let pos_command:String = match go_job.position {
			Startpos => "position startpos".to_string(),
			Fen{ fen } => format!("position fen {}", fen),
			StartposAndMovesStr{ moves_str } => format!("position startpos moves {}", moves_str),
			FenAndMovesStr{ fen, moves_str } => format!("position fen {} moves {}", fen, moves_str),
		};
		
		let _ = self.issue_command(pos_command).await?;
		
		let mut go_command = "go".to_string();
		
		for (key, value) in go_job.go_options {
			go_command = go_command + &format!(" {} {}", key, value);
		}
		
		let _ = self.issue_command(go_command).await?;
		
		let result = self.rx.recv();
		
		let mut bestmove:Option<String> = None;
		let mut ponder:Option<String> = None;
		
		if let Ok(result) = result {
			let parts:Vec<&str> = result.split(" ").collect();
		
			if parts.len() > 1 {
				bestmove = Some(parts[1].to_string());
			}

			if parts.len() > 3 {
				ponder = Some(parts[3].to_string());
			}
		}
		
		Ok(GoResult {
			bestmove: bestmove,
			ponder: ponder,
		})
	}
}
