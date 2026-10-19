use std::fmt;

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

/// Convert one complete game-engine turn into a board and a piece mask.
pub fn parse_input(input: &str) -> Result<GameState, ParseError> {
    let lines: Vec<&str> = input.lines().collect();
    let player = if lines.iter().any(|line| line.contains("exec p2")) {
        Player::Two
    } else {
        Player::One
    };

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
    let Some(piece_width) = state.piece.first().map(Vec::len) else {
        return false;
    };
    if piece_width == 0 || state.piece.iter().any(|row| row.len() != piece_width) {
        return false;
    }
    if y.checked_add(piece_height)
        .is_none_or(|end| end > board_height)
        || x.checked_add(piece_width)
            .is_none_or(|end| end > board_width)
    {
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

fn placement_score(state: &GameState, x: usize, y: usize) -> f64 {
    let width = state.width();
    let height = state.height();
    let own = state.player.own_markers();
    let opponent = state.player.opponent_markers();
    let opponents: Vec<(usize, usize)> = state
        .board
        .iter()
        .enumerate()
        .flat_map(|(row, cells)| {
            cells
                .iter()
                .enumerate()
                .filter_map(move |(col, cell)| opponent.contains(cell).then_some((col, row)))
        })
        .collect();

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

            if !opponents.is_empty() {
                let distance = opponents
                    .iter()
                    .map(|(ox, oy)| bx.abs_diff(*ox) + by.abs_diff(*oy))
                    .min()
                    .unwrap_or(0);
                // Advance toward the opponent while still valuing room for later pieces.
                score -= distance as f64 * 0.34;
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

    let mut best: Option<((usize, usize), f64)> = None;
    for y in 0..=state.height() - piece_height {
        for x in 0..=state.width() - piece_width {
            if !is_legal_placement(state, x, y) {
                continue;
            }
            let score = placement_score(state, x, y);
            if best.is_none_or(|(_, best_score)| score > best_score) {
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
}
