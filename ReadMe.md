# uciengine

Rust uci engine wrapper.

# Usage

```rust
use uciengine::uciengine::*;
use uciengine::uciengine::Position::*;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
	let go_job = GoJob::new()				
		.uci_opt("UCI_Variant".to_string(), "atomic".to_string())
		.uci_opt("Hash".to_string(), "128".to_string())
		.uci_opt("Threads".to_string(), "4".to_string())
		.pos(FenAndMovesStr{
			fen: "k7/8/8/8/8/8/R7/7K w - - 0 1".to_string(),
			moves_str: "h1h2".to_string()
		})
		.tc(Timecontrol{
			wtime: 15000,
			winc: 0,
			btime: 15000,
			binc: 0
		})
	;
	
	let mut uciengine = UciEngine::new("./stockfish12".to_string());
	
	let result = uciengine.go(go_job).await?;
	
	println!("result {:?}", result);
	
	Ok(())
}
```
