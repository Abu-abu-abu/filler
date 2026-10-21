use std::io::{self, Write};

/// Works with both ways the engine may run a robot:
/// one process per turn (input ends after the piece) or one long-lived
/// process that receives a new Anfield + Piece every turn.
fn main() {
    let stdin = io::stdin();
    let mut reader = stdin.lock();
    let stdout = io::stdout();
    let mut out = stdout.lock();

    // The "$$$ exec pN" line may be sent only once, so the player is remembered.
    let mut player = filler::Player::One;
    let mut answered = false;

    loop {
        match filler::read_turn(&mut reader, &mut player) {
            Ok(Some(turn)) => {
                let placement = filler::parse_input_as(&turn, player)
                    .ok()
                    .and_then(|state| filler::choose_placement(&state));
                answered = true;
                if out
                    .write_all(filler::format_output(placement).as_bytes())
                    .is_err()
                    || out.flush().is_err()
                {
                    return;
                }
            }
            Ok(None) => break,
            Err(_) => {
                let _ = out.write_all(filler::format_output(None).as_bytes());
                let _ = out.flush();
                return;
            }
        }
    }

    if !answered {
        let _ = out.write_all(filler::format_output(None).as_bytes());
        let _ = out.flush();
    }
}
