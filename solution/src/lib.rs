use std::fmt;
use std::io::{self, BufRead};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Player {
    One,
    Two,
}

impl Player {
    pub fn from_id(id: u8) -> Self {
        if id == 2 {
            Self::Two
        } else {
            Self::One
        }
    }

    fn own_markers(self) -> [char; 2] {
        match self {
            Self::One => ['@', 'a'],
            Self::Two => ['$', 's'],
        }
    }

    fn opponent_markers(self) -> [char; 2] {
        match self {
            Self::One => ['$', 's'],
            Self::Two => ['@', 'a'],
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GameState {
    pub player: Player,
    pub board: Vec<Vec<char>>,
    /// `true` marks a filled cell in the incoming piece.
    pub piece: Vec<Vec<bool>>,
}

impl GameState {
    pub fn width(&self) -> usize {
        self.board.first().map_or(0, Vec::len)
    }

    pub fn height(&self) -> usize {
        self.board.len()
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ParseError(pub String);

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for ParseError {}

fn parse_dimensions(line: &str, header: &str) -> Option<(usize, usize)> {
    let mut words = line.split_whitespace();
    if words.next()? != header {
        return None;
    }
    let width = words.next()?.trim_end_matches(':').parse().ok()?;
    let height = words.next()?.trim_end_matches(':').parse().ok()?;
    Some((width, height))
}

fn parse_board_row(line: &str, width: usize) -> Option<Vec<char>> {
    let line = line.trim_start();
    let prefix_len = line.chars().take_while(char::is_ascii_digit).count();
    if prefix_len == 0 {
        return None;
    }
    let cells = line[prefix_len..].trim_start();
    let row: Vec<char> = cells.chars().collect();
    (row.len() == width).then_some(row)
}

/// Extract the player from a `$$$ exec p<number> : [<path>]` line.
pub fn parse_player_line(line: &str) -> Option<Player> {
    let rest = &line[line.find("exec p")? + "exec p".len()..];
    match rest.chars().next()? {
        '1' => Some(Player::One),
        '2' => Some(Player::Two),
        _ => None,
    }
}

/// Read one complete turn (optional `$$$ exec` line, Anfield, Piece) from a
/// stream. Updates `player` when an `exec` line is seen, so the number sent
/// only on the first turn is remembered. Returns `Ok(None)` at end of input.
pub fn read_turn<R: BufRead>(reader: &mut R, player: &mut Player) -> io::Result<Option<String>> {
    let mut turn = String::new();
    let mut piece_rows_left: Option<usize> = None;
    let mut line = String::new();

    loop {
        line.clear();
        if reader.read_line(&mut line)? == 0 {
            return Ok((!turn.trim().is_empty()).then_some(turn));
        }
        if let Some(found) = parse_player_line(&line) {
            *player = found;
        }
        match piece_rows_left {
            None => {
                turn.push_str(&line);
                if line.trim_start().starts_with("Piece ") {
                    let rows = parse_dimensions(line.trim_start(), "Piece").map_or(0, |(_, h)| h);
                    if rows == 0 {
                        return Ok(Some(turn));
                    }
                    piece_rows_left = Some(rows);
                }
            }
            Some(left) => {
                if line.trim().is_empty() {
                    continue;
                }
                turn.push_str(&line);
                if left <= 1 {
                    return Ok(Some(turn));
                }
                piece_rows_left = Some(left - 1);
            }
        }
    }
}

/// Convert one complete game-engine turn into a board and a piece mask.
/// The player is taken from the `exec` line, or is `Player::One` if absent.
pub fn parse_input(input: &str) -> Result<GameState, ParseError> {
    parse_input_as(input, Player::One)
}

/// Like [`parse_input`], but `fallback` is used when the turn has no `exec` line.
pub fn parse_input_as(input: &str, fallback: Player) -> Result<GameState, ParseError> {
    let lines: Vec<&str> = input.lines().collect();
    let player = lines
        .iter()
        .find_map(|line| parse_player_line(line))
        .unwrap_or(fallback);

    let board_header = lines
        .iter()
        .position(|line| {
            line.trim_start().starts_with("Anfield ") || line.trim_start().starts_with("Plateau ")
        })
        .ok_or_else(|| ParseError("missing Anfield header".into()))?;
    let header = lines[board_header].trim_start();
    let dims = parse_dimensions(
        header,
        if header.starts_with("Plateau ") {
            "Plateau"
        } else {
            "Anfield"
        },
    )
    .ok_or_else(|| ParseError("invalid Anfield dimensions".into()))?;
    let (width, height) = dims;
    if width == 0 || height == 0 {
        return Err(ParseError("Anfield dimensions must be non-zero".into()));
    }

    let piece_header = lines
        .iter()
        .position(|line| line.trim_start().starts_with("Piece "))
        .ok_or_else(|| ParseError("missing Piece header".into()))?;
    if piece_header <= board_header {
        return Err(ParseError(
            "Piece header appears before Anfield rows".into(),
        ));
    }

    let mut board = Vec::with_capacity(height);
    for line in &lines[board_header + 1..piece_header] {
        if let Some(row) = parse_board_row(line, width) {
            board.push(row);
        }
    }
    if board.len() != height {
        return Err(ParseError(format!(
            "expected {height} Anfield rows, found {}",
            board.len()
        )));
    }

    let piece_dims = parse_dimensions(lines[piece_header].trim_start(), "Piece")
        .ok_or_else(|| ParseError("invalid Piece dimensions".into()))?;
    let (piece_width, piece_height) = piece_dims;
    if piece_width == 0 || piece_height == 0 {
        return Err(ParseError("Piece dimensions must be non-zero".into()));
    }

    let mut piece: Vec<Vec<bool>> = Vec::with_capacity(piece_height);
    for line in lines[piece_header + 1..]
        .iter()
        .filter(|line| !line.trim().is_empty())
        .take(piece_height)
    {
        let cells: Vec<char> = line.trim().chars().take(piece_width).collect();
        if cells.len() != piece_width {
            return Err(ParseError(
                "piece row is shorter than its declared width".into(),
            ));
        }
        piece.push(cells.into_iter().map(|cell| cell != '.').collect());
    }
    if piece.len() != piece_height {
        return Err(ParseError(format!(
            "expected {piece_height} piece rows, found {}",
            piece.len()
        )));
    }
    if !piece.iter().flatten().any(|cell| *cell) {
        return Err(ParseError("piece contains no filled cells".into()));
    }

    Ok(GameState {
        player,
        board,
        piece,
    })
}

/// Return whether placing the piece at its top-left coordinate follows Filler rules.
pub fn is_legal_placement(state: &GameState, x: usize, y: usize) -> bool {
    let board_height = state.height();
    let board_width = state.width();
    let piece_height = state.piece.len();
    let piece_width = match state.piece.first().map(Vec::len) {
        Some(width) => width,
        None => return false,
    };
    if piece_width == 0 || state.piece.iter().any(|row| row.len() != piece_width) {
        return false;
    }
    let fits_vertically = y
        .checked_add(piece_height)
        .map_or(false, |end| end <= board_height);
    let fits_horizontally = x
        .checked_add(piece_width)
        .map_or(false, |end| end <= board_width);
    if !fits_vertically || !fits_horizontally {
        return false;
    }

    let own = state.player.own_markers();
    let mut own_overlaps = 0usize;
    for (dy, row) in state.piece.iter().enumerate() {
        for (dx, filled) in row.iter().enumerate() {
            if !filled {
                continue;
            }
            let cell = state.board[y + dy][x + dx];
            if own.contains(&cell) {
                own_overlaps += 1;
                if own_overlaps > 1 {
                    return false;
                }
            } else if cell != '.' {
                // Opponent territory and any other occupied map cell are blocked.
                return false;
            }
        }
    }
    own_overlaps == 1
}

/// Manhattan distance from every cell to the nearest opponent cell, computed
/// once per turn in O(width * height). `None` when there is no opponent cell.
fn opponent_distances(state: &GameState) -> Option<Vec<Vec<u32>>> {
    let width = state.width();
    let height = state.height();
    let opponent = state.player.opponent_markers();
    let far = u32::MAX / 2;
    let mut dist = vec![vec![far; width]; height];
    let mut found = false;
    for (y, row) in state.board.iter().enumerate() {
        for (x, cell) in row.iter().enumerate() {
            if opponent.contains(cell) {
                dist[y][x] = 0;
                found = true;
            }
        }
    }
    if !found {
        return None;
    }
    for y in 0..height {
        for x in 0..width {
            if y > 0 {
                dist[y][x] = dist[y][x].min(dist[y - 1][x] + 1);
            }
            if x > 0 {
                dist[y][x] = dist[y][x].min(dist[y][x - 1] + 1);
            }
        }
    }
    for y in (0..height).rev() {
        for x in (0..width).rev() {
            if y + 1 < height {
                dist[y][x] = dist[y][x].min(dist[y + 1][x] + 1);
            }
            if x + 1 < width {
                dist[y][x] = dist[y][x].min(dist[y][x + 1] + 1);
            }
        }
    }
    Some(dist)
}

fn placement_score(
    state: &GameState,
    distances: Option<&Vec<Vec<u32>>>,
    x: usize,
    y: usize,
) -> f64 {
    let width = state.width();
    let height = state.height();
    let own = state.player.own_markers();
    let opponent = state.player.opponent_markers();

    let mut score = 0.0;
    for (dy, row) in state.piece.iter().enumerate() {
        for (dx, filled) in row.iter().enumerate() {
            if !filled {
                continue;
            }
            let bx = x + dx;
            let by = y + dy;
            let cell = state.board[by][bx];
            if own.contains(&cell) {
                continue;
            }

            if let Some(dist) = distances {
                // Advance toward the opponent while still valuing room for later pieces.
                score -= f64::from(dist[by][bx]) * 0.34;
            } else {
                let center_x = (width.saturating_sub(1)) as f64 / 2.0;
                let center_y = (height.saturating_sub(1)) as f64 / 2.0;
                score -= ((bx as f64 - center_x).abs() + (by as f64 - center_y).abs()) * 0.12;
            }

            for (nx, ny) in [
                (bx.wrapping_sub(1), by),
                (bx + 1, by),
                (bx, by.wrapping_sub(1)),
                (bx, by + 1),
            ] {
                if nx >= width || ny >= height {
                    continue;
                }
                let neighbor = state.board[ny][nx];
                if opponent.contains(&neighbor) {
                    score += 5.0;
                } else if neighbor == '.' {
                    score += 0.30;
                }
            }

            // A small center preference preserves usable space near the edges.
            let edge_clearance = bx.min(width - 1 - bx).min(by.min(height - 1 - by));
            score += edge_clearance as f64 * 0.035;
        }
    }
    score
}

/// Select the strongest currently legal placement. Coordinates are `(x, y)`.
pub fn choose_placement(state: &GameState) -> Option<(usize, usize)> {
    let piece_height = state.piece.len();
    let piece_width = state.piece.first()?.len();
    if piece_height == 0
        || piece_width == 0
        || piece_height > state.height()
        || piece_width > state.width()
    {
        return None;
    }

    let distances = opponent_distances(state);
    let mut best: Option<((usize, usize), f64)> = None;
    for y in 0..=state.height() - piece_height {
        for x in 0..=state.width() - piece_width {
            if !is_legal_placement(state, x, y) {
                continue;
            }
            let score = placement_score(state, distances.as_ref(), x, y);
            let better = match best {
                None => true,
                Some((_, best_score)) => score > best_score,
            };
            if better {
                best = Some(((x, y), score));
            }
        }
    }
    best.map(|(position, _)| position)
}

/// Format the one-line response expected by game_engine.
pub fn format_output(placement: Option<(usize, usize)>) -> String {
    match placement {
        Some((x, y)) => format!("{x} {y}\n"),
        None => "0 0\n".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = "$$$ exec p1 : [robots/bender]\nAnfield 8 5:\n    01234567\n000 ........\n001 ........\n002 ..@.....\n003 ........\n004 ......$.\nPiece 4 2:\n.OO.\n..O.\n";

    #[test]
    fn parses_anfield_piece_and_player_from_engine_input() {
        let state = parse_input(SAMPLE).unwrap();
        assert_eq!((state.width(), state.height()), (8, 5));
        assert_eq!(state.player, Player::One);
        assert_eq!(state.board[2][2], '@');
        assert_eq!(state.board[4][6], '$');
        assert_eq!(
            state.piece,
            vec![
                vec![false, true, true, false],
                vec![false, false, true, false]
            ]
        );
    }

    #[test]
    fn parses_player_two_and_last_piece_markers() {
        let input = SAMPLE
            .replace("exec p1", "exec p2")
            .replace("..@.....", "..a.....")
            .replace("......$.", "......s.");
        let state = parse_input(&input).unwrap();
        assert_eq!(state.player, Player::Two);
        assert_eq!(state.board[2][2], 'a');
        assert_eq!(state.board[4][6], 's');
    }

    #[test]
    fn accepts_exactly_one_own_overlap_and_no_opponent_overlap() {
        let state = parse_input(SAMPLE).unwrap();
        assert!(is_legal_placement(&state, 0, 2));
        assert!(!is_legal_placement(&state, 4, 2)); // misses own territory
        assert!(!is_legal_placement(&state, 4, 3)); // overlaps the opponent
    }

    #[test]
    fn rejects_two_own_overlaps() {
        let input =
            "$$$ exec p1 : [bot]\nAnfield 5 3:\n000 .....\n001 .@@..\n002 .....\nPiece 3 1:\nOOO\n";
        let state = parse_input(input).unwrap();
        assert!(!is_legal_placement(&state, 0, 1));
    }

    #[test]
    fn never_selects_a_piece_outside_the_board() {
        let input = "$$$ exec p1 : [bot]\nAnfield 4 3:\n000 ....\n001 .@..\n002 ....\nPiece 3 2:\n.O.\n..O\n";
        let state = parse_input(input).unwrap();
        let position = choose_placement(&state).expect("a legal in-bounds placement exists");
        assert!(position.0 + 3 <= state.width());
        assert!(position.1 + 2 <= state.height());
        assert!(is_legal_placement(&state, position.0, position.1));
    }

    #[test]
    fn rejects_placements_that_extend_past_every_edge() {
        let state = parse_input(SAMPLE).unwrap();
        assert!(!is_legal_placement(&state, 5, 2)); // past the right edge
        assert!(!is_legal_placement(&state, 0, 4)); // past the bottom edge
        assert!(!is_legal_placement(&state, usize::MAX, 0)); // overflow-safe
    }

    #[test]
    fn output_is_exactly_x_space_y_newline() {
        assert_eq!(format_output(Some((7, 2))), "7 2\n");
        assert_eq!(format_output(None), "0 0\n");
    }

    #[test]
    fn rejects_malformed_input_without_panicking() {
        assert!(parse_input("Anfield bad input").is_err());
    }

    #[test]
    fn reports_no_move_when_every_placement_is_blocked() {
        let input = "$$$ exec p1 : [bot]\nAnfield 3 1:\n000 .@$\nPiece 3 1:\nOOO\n";
        let state = parse_input(input).unwrap();
        assert_eq!(choose_placement(&state), None);
        assert_eq!(format_output(choose_placement(&state)), "0 0\n");
    }

    #[test]
    fn reads_consecutive_turns_and_remembers_the_player() {
        let first = "$$$ exec p2 : [bot]\nAnfield 3 1:\n    012\n000 .s@\nPiece 1 1:\nO\n";
        let second = "Anfield 3 1:\n    012\n000 .$@\nPiece 1 1:\nO\n";
        let mut reader = io::Cursor::new(format!("{first}{second}"));
        let mut player = Player::One;

        let turn = read_turn(&mut reader, &mut player).unwrap().unwrap();
        assert_eq!(player, Player::Two);
        assert_eq!(parse_input_as(&turn, player).unwrap().player, Player::Two);

        let turn = read_turn(&mut reader, &mut player).unwrap().unwrap();
        assert!(!turn.contains("exec"));
        assert_eq!(parse_input_as(&turn, player).unwrap().player, Player::Two);

        assert!(read_turn(&mut reader, &mut player).unwrap().is_none());
    }

    #[test]
    fn chooses_a_legal_move_for_player_two_without_an_exec_line() {
        let turn = "Anfield 4 1:\n000 .$..\nPiece 2 1:\nOO\n";
        let state = parse_input_as(turn, Player::Two).unwrap();
        assert_eq!(state.player, Player::Two);
        let (x, y) = choose_placement(&state).expect("legal move");
        assert!(is_legal_placement(&state, x, y));
    }

    #[test]
    fn prefers_moves_toward_the_opponent() {
        let input = "$$$ exec p1 : [bot]\nAnfield 9 1:\n000 ..@.....$\nPiece 2 1:\nOO\n";
        let state = parse_input(input).unwrap();
        // Right of the own cell moves toward the opponent; left moves away.
        assert_eq!(choose_placement(&state), Some((2, 0)));
    }
}
