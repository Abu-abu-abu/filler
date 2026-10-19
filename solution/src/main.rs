use std::io::{self, Read};

fn main() {
    let mut input = String::new();
    if io::stdin().read_to_string(&mut input).is_err() {
        println!("0 0");
        return;
    }
    let response = filler::parse_input(&input)
        .ok()
        .and_then(|state| filler::choose_placement(&state));
    print!("{}", filler::format_output(response));
}
