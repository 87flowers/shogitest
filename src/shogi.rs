use std::fmt;
use std::ops::Not;

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum Color {
    Sente,
    Gote,
}

impl Color {
    pub fn to_index(self) -> usize {
        match self {
            Color::Sente => 0,
            Color::Gote => 1,
        }
    }

    pub fn parse(s: &str) -> Option<Color> {
        match s {
            "b" => Some(Color::Sente),
            "w" => Some(Color::Gote),
            _ => None,
        }
    }
}

impl fmt::Display for Color {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Color::Sente => "b",
                Color::Gote => "w",
            }
        )
    }
}

impl Not for Color {
    type Output = Self;

    fn not(self) -> Self::Output {
        match self {
            Color::Sente => Color::Gote,
            Color::Gote => Color::Sente,
        }
    }
}

#[derive(Copy, Clone, PartialEq, Eq, Debug, Default)]
#[repr(transparent)]
pub struct Square(u8);

impl Square {
    pub fn new(file: i8, rank: i8) -> Square {
        let value = rank * 9 + file;
        assert!(value < 81);
        Square(value as u8)
    }

    pub fn parse(c0: u8, c1: u8) -> Option<Square> {
        if c0 < b'1' || c0 > b'9' || c1 < b'a' || c1 > b'i' {
            return None;
        }
        let file = c0 - b'1';
        let rank = c1 - b'a';
        Some(Self::new(file as i8, rank as i8))
    }

    pub fn from_fen_ordering(i: usize) -> Square {
        let file = (8 - i % 9) as i8;
        let rank = (i / 9) as i8;
        Square::new(file, rank)
    }

    pub fn file(self) -> i8 {
        (self.0 % 9) as i8
    }

    pub fn rank(self) -> i8 {
        (self.0 / 9) as i8
    }

    pub fn to_index(self) -> usize {
        self.0 as usize
    }
}

impl fmt::Display for Square {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}{}",
            (b'1' + (self.file() as u8)) as char,
            (b'a' + (self.rank() as u8)) as char,
        )
    }
}

#[derive(Copy, Clone, PartialEq, Eq, Debug, Default)]
pub struct Delta {
    file: i8,
    rank: i8,
}

impl Delta {
    pub fn new(file: i8, rank: i8) -> Delta {
        Delta { file, rank }
    }

    pub fn normalize_to_sente(self, piece_color: Color) -> Delta {
        match piece_color {
            Color::Sente => self,
            Color::Gote => Delta {
                file: -self.file,
                rank: -self.rank,
            },
        }
    }

    pub fn is_ring(self) -> bool {
        self.file != 0 && self.rank != 0 && self.file.abs() <= 1 && self.rank.abs() <= 1
    }
}

#[derive(Copy, Clone, PartialEq, Eq, Debug, Default)]
#[repr(u8)]
pub enum PieceType {
    #[default]
    None = 0o00,
    Pawn = 0o01,
    Bishop = 0o02,
    Rook = 0o03,
    Lance = 0o04,
    Knight = 0o05,
    Silver = 0o06,
    Gold = 0o07,
    King = 0o10,
    Tokin = 0o11,
    Horse = 0o12,
    Dragon = 0o13,
    NariLance = 0o14,
    NariKnight = 0o15,
    NariSilver = 0o16,
}

impl PieceType {
    pub fn promotable(self) -> bool {
        self as u8 >= Self::Pawn as u8 && self as u8 <= Self::Silver as u8
    }

    pub fn promoted(self) -> bool {
        self as u8 >= Self::Tokin as u8
    }

    pub fn promote(self) -> PieceType {
        if self == Self::None || self == Self::Gold {
            self
        } else {
            // SAFETY: Unpromotables have been excluded above; 0o17 can never be generated here
            unsafe { std::mem::transmute::<u8, PieceType>((self as u8) | 0o10) }
        }
    }

    pub fn demote(self) -> PieceType {
        if !self.promoted() {
            self
        } else {
            // SAFETY: Unpromoteds have been excluded above; all values between 0o00 and 0o07 are valid
            unsafe { std::mem::transmute::<u8, PieceType>((self as u8) & 0o07) }
        }
    }

    pub fn to_str(self, color: Color) -> &'static str {
        match (color, self) {
            (Color::Sente, PieceType::None) => "",
            (Color::Sente, PieceType::Pawn) => "P",
            (Color::Sente, PieceType::Bishop) => "B",
            (Color::Sente, PieceType::Rook) => "R",
            (Color::Sente, PieceType::Lance) => "L",
            (Color::Sente, PieceType::Knight) => "N",
            (Color::Sente, PieceType::Silver) => "S",
            (Color::Sente, PieceType::Gold) => "G",
            (Color::Sente, PieceType::King) => "K",
            (Color::Sente, PieceType::Tokin) => "+P",
            (Color::Sente, PieceType::Horse) => "+B",
            (Color::Sente, PieceType::Dragon) => "+R",
            (Color::Sente, PieceType::NariLance) => "+L",
            (Color::Sente, PieceType::NariKnight) => "+N",
            (Color::Sente, PieceType::NariSilver) => "+S",
            (Color::Gote, PieceType::None) => "",
            (Color::Gote, PieceType::Pawn) => "p",
            (Color::Gote, PieceType::Bishop) => "b",
            (Color::Gote, PieceType::Rook) => "r",
            (Color::Gote, PieceType::Lance) => "l",
            (Color::Gote, PieceType::Knight) => "n",
            (Color::Gote, PieceType::Silver) => "s",
            (Color::Gote, PieceType::Gold) => "g",
            (Color::Gote, PieceType::King) => "k",
            (Color::Gote, PieceType::Tokin) => "+p",
            (Color::Gote, PieceType::Horse) => "+b",
            (Color::Gote, PieceType::Dragon) => "+r",
            (Color::Gote, PieceType::NariLance) => "+l",
            (Color::Gote, PieceType::NariKnight) => "+n",
            (Color::Gote, PieceType::NariSilver) => "+s",
        }
    }
}

impl fmt::Display for PieceType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_str(Color::Sente))
    }
}

#[derive(Copy, Clone, PartialEq, Eq, Debug, Default)]
pub enum Move {
    #[default]
    None,
    Win,
    Resign,
    Drop(PieceType, Square),
    Move {
        from: Square,
        to: Square,
        promo: bool,
    },
}

impl Move {
    pub fn parse(s: &str) -> Option<Move> {
        if s.eq_ignore_ascii_case("null") {
            return Some(Move::None);
        }
        if s.eq_ignore_ascii_case("win") {
            return Some(Move::Win);
        }
        if s.eq_ignore_ascii_case("resign") {
            return Some(Move::Resign);
        }
        let bytes = s.as_bytes();
        if bytes.len() < 4 || bytes.len() > 5 {
            return None;
        }
        if bytes[1] == b'*' {
            if bytes.len() != 4 {
                return None;
            }
            let pt = match bytes[0] {
                b'P' => PieceType::Pawn,
                b'N' => PieceType::Knight,
                b'L' => PieceType::Lance,
                b'S' => PieceType::Silver,
                b'G' => PieceType::Gold,
                b'B' => PieceType::Bishop,
                b'R' => PieceType::Rook,
                _ => return None,
            };
            let to = Square::parse(bytes[2], bytes[3])?;
            Some(Move::Drop(pt, to))
        } else {
            if s.len() == 5 && bytes[4] != b'+' {
                return None;
            }
            let promo = s.len() == 5;
            let from = Square::parse(bytes[0], bytes[1])?;
            let to = Square::parse(bytes[2], bytes[3])?;
            Some(Move::Move { from, to, promo })
        }
    }
}

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub struct Place(Color, PieceType);

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub struct Hand {
    rook: u8,
    bishop: u8,
    gold: u8,
    silver: u8,
    knight: u8,
    lance: u8,
    pawn: u8,
}

impl Default for Hand {
    fn default() -> Self {
        Hand {
            rook: 0,
            bishop: 0,
            gold: 0,
            silver: 0,
            knight: 0,
            lance: 0,
            pawn: 0,
        }
    }
}

impl Hand {
    pub fn get(&self, pt: PieceType) -> &u8 {
        match pt {
            PieceType::Rook => &self.rook,
            PieceType::Bishop => &self.bishop,
            PieceType::Gold => &self.gold,
            PieceType::Silver => &self.silver,
            PieceType::Knight => &self.knight,
            PieceType::Lance => &self.lance,
            PieceType::Pawn => &self.pawn,
            _ => panic!(),
        }
    }

    fn from_parse(&mut self, pt: PieceType, modifier: Option<usize>) {
        let count = modifier.unwrap_or_else(|| 1) as u8;
        match pt {
            PieceType::Rook => self.rook = count,
            PieceType::Bishop => self.bishop = count,
            PieceType::Gold => self.gold = count,
            PieceType::Silver => self.silver = count,
            PieceType::Knight => self.knight = count,
            PieceType::Lance => self.lance = count,
            PieceType::Pawn => self.pawn = count,
            _ => panic!(),
        }
    }
}

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub struct Position {
    board: [Place; 81],
    hand: [Hand; 2],
    stm: Color,
    ply: usize,
}

impl Default for Position {
    fn default() -> Self {
        Position::parse("lnsgkgsnl/1r5b1/ppppppppp/9/9/9/PPPPPPPPP/1B5R1/LNSGKGSNL b - 1").unwrap()
    }
}

impl Position {
    pub fn is_clone_of(&self, other: &Position) -> bool {
        self.board == other.board && self.hand == other.hand && self.stm == other.stm
    }

    pub fn king_sq(&self, king_color: Color) -> Square {
        Square(
            self.board
                .iter()
                .position(|&p| p == Place(king_color, PieceType::King))
                .unwrap() as u8,
        )
    }

    // pub fn is_in_check(&self) -> bool {
    //     self.get_checkers() != 0
    // }

    pub fn parse(s: &str) -> Option<Position> {
        let mut it = s.split(' ');
        let board = it.next()?;
        let color = it.next()?;
        let hand = it.next()?;
        let ply = it.next()?;
        if it.next() == None {
            Position::parse_parts(board, color, hand, ply)
        } else {
            None
        }
    }

    pub fn parse_parts(board: &str, color: &str, hand: &str, ply: &str) -> Option<Position> {
        Some(Position {
            board: Position::parse_board(board)?,
            hand: Position::parse_hand(hand)?,
            stm: Color::parse(color)?,
            ply: ply.parse().ok()?,
        })
    }

    fn parse_board(s: &str) -> Option<[Place; 81]> {
        let mut board = [Place(Color::Sente, PieceType::None); 81];

        let board_str = s.as_bytes();
        let mut place_index: usize = 0;
        let mut i: usize = 0;

        while place_index < 81 && i < board_str.len() {
            let sq = Square::from_fen_ordering(place_index);
            let ch = board_str[i];
            match ch {
                b'/' => {
                    if sq.file() != 8 || place_index == 0 {
                        return None;
                    }
                    i += 1;
                    continue;
                }
                b'1'..=b'9' => {
                    place_index += (ch - b'0') as usize;
                    i += 1;
                    continue;
                }
                b'p' => board[sq.to_index()] = Place(Color::Gote, PieceType::Pawn),
                b'b' => board[sq.to_index()] = Place(Color::Gote, PieceType::Bishop),
                b'r' => board[sq.to_index()] = Place(Color::Gote, PieceType::Rook),
                b'l' => board[sq.to_index()] = Place(Color::Gote, PieceType::Lance),
                b'n' => board[sq.to_index()] = Place(Color::Gote, PieceType::Knight),
                b's' => board[sq.to_index()] = Place(Color::Gote, PieceType::Silver),
                b'g' => board[sq.to_index()] = Place(Color::Gote, PieceType::Gold),
                b'k' => board[sq.to_index()] = Place(Color::Gote, PieceType::King),
                b'P' => board[sq.to_index()] = Place(Color::Sente, PieceType::Pawn),
                b'B' => board[sq.to_index()] = Place(Color::Sente, PieceType::Bishop),
                b'R' => board[sq.to_index()] = Place(Color::Sente, PieceType::Rook),
                b'L' => board[sq.to_index()] = Place(Color::Sente, PieceType::Lance),
                b'N' => board[sq.to_index()] = Place(Color::Sente, PieceType::Knight),
                b'S' => board[sq.to_index()] = Place(Color::Sente, PieceType::Silver),
                b'G' => board[sq.to_index()] = Place(Color::Sente, PieceType::Gold),
                b'K' => board[sq.to_index()] = Place(Color::Sente, PieceType::King),
                b'+' => {
                    i += 1;
                    if i >= board_str.len() {
                        return None;
                    }
                    match board_str[i] {
                        b'p' => board[sq.to_index()] = Place(Color::Gote, PieceType::Tokin),
                        b'b' => board[sq.to_index()] = Place(Color::Gote, PieceType::Horse),
                        b'r' => board[sq.to_index()] = Place(Color::Gote, PieceType::Dragon),
                        b'l' => board[sq.to_index()] = Place(Color::Gote, PieceType::NariLance),
                        b'n' => board[sq.to_index()] = Place(Color::Gote, PieceType::NariKnight),
                        b's' => board[sq.to_index()] = Place(Color::Gote, PieceType::NariSilver),
                        b'P' => board[sq.to_index()] = Place(Color::Sente, PieceType::Tokin),
                        b'B' => board[sq.to_index()] = Place(Color::Sente, PieceType::Horse),
                        b'R' => board[sq.to_index()] = Place(Color::Sente, PieceType::Dragon),
                        b'L' => board[sq.to_index()] = Place(Color::Sente, PieceType::NariLance),
                        b'N' => board[sq.to_index()] = Place(Color::Sente, PieceType::NariKnight),
                        b'S' => board[sq.to_index()] = Place(Color::Sente, PieceType::NariSilver),
                        _ => return None,
                    }
                }
                _ => return None,
            }
            i += 1;
            place_index += 1;
        }

        if place_index != 81 || i != board_str.len() {
            return None;
        }

        Some(board)
    }

    fn parse_hand(s: &str) -> Option<[Hand; 2]> {
        let mut hand = [Hand::default(); 2];

        if s == "-" {
            return Some(hand);
        }

        let mut modifier: Option<usize> = None;

        for ch in s.bytes() {
            match ch {
                b'0'..=b'9' => {
                    if modifier == None && ch == b'0' {
                        return None;
                    }
                    modifier = Some(modifier.unwrap_or_else(|| 0) * 10 + (ch - b'0') as usize);
                    if modifier > Some(18) {
                        return None;
                    }
                    continue;
                }
                b'p' => hand[Color::Gote.to_index()].from_parse(PieceType::Pawn, modifier),
                b'b' => hand[Color::Gote.to_index()].from_parse(PieceType::Bishop, modifier),
                b'r' => hand[Color::Gote.to_index()].from_parse(PieceType::Rook, modifier),
                b'l' => hand[Color::Gote.to_index()].from_parse(PieceType::Lance, modifier),
                b'n' => hand[Color::Gote.to_index()].from_parse(PieceType::Knight, modifier),
                b's' => hand[Color::Gote.to_index()].from_parse(PieceType::Silver, modifier),
                b'g' => hand[Color::Gote.to_index()].from_parse(PieceType::Gold, modifier),
                b'P' => hand[Color::Sente.to_index()].from_parse(PieceType::Pawn, modifier),
                b'B' => hand[Color::Sente.to_index()].from_parse(PieceType::Bishop, modifier),
                b'R' => hand[Color::Sente.to_index()].from_parse(PieceType::Rook, modifier),
                b'L' => hand[Color::Sente.to_index()].from_parse(PieceType::Lance, modifier),
                b'N' => hand[Color::Sente.to_index()].from_parse(PieceType::Knight, modifier),
                b'S' => hand[Color::Sente.to_index()].from_parse(PieceType::Silver, modifier),
                b'G' => hand[Color::Sente.to_index()].from_parse(PieceType::Gold, modifier),
                _ => return None,
            }
            modifier = None;
        }

        if modifier != None {
            return None;
        }

        Some(hand)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn move_parse_test() {
        assert_eq!(
            Move::parse("1a2b+").unwrap(),
            Move::Move {
                from: Square::parse(b'1', b'a').unwrap(),
                to: Square::parse(b'2', b'b').unwrap(),
                promo: true
            }
        )
    }

    #[test]
    fn print_default_board() {
        eprintln!("{:?}", Position::default())
    }
}
