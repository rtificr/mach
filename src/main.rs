use std::cmp::PartialEq;
use std::collections::HashMap;
use std::{env, io};
use std::fs::File;
use std::io::{Read, Write};
use colored::Colorize;
use fancy_regex::Regex;
use std::process::Command;

const DEFAULT_MAX_ITERS: u16 = 32768;

fn main() {
    let path = env::args().nth(1).unwrap_or(String::from("program.rt.mach"));
    let mut file = File::open(path).expect("File not found");
    let mut fstr = String::new();

    file.read_to_string(&mut fstr).unwrap();
    let meta_regex = Regex::new(r"(?<=#).*(?=\n)").expect("Invalid regex");
    let metas = meta_regex.find_iter(&fstr)
        .map(|m| m.expect("Error when looking for meta").as_str()).collect::<Vec<_>>();

    let dim_regex = Regex::new(r"^\d+\*\d+$").expect("Invalid regex");
    let iter_regex = Regex::new(r"^(?:iters?\s*)(\d+)$").expect("Invalid regex");
    let delay_regex = Regex::new(r"^(?:delay\s*)(\d+)$").expect("Invalid regex");
    let print_regex = Regex::new(r"^(?:print)$").expect("Invalid regex");

    let mut dims = None;
    let mut max_iters = DEFAULT_MAX_ITERS;
    let mut delay = None;
    let mut print = false;

    for m in metas {
        if dim_regex.is_match(m).expect("Error when looking for dimensions") {
            dims = Some({
                let arr = m.split("*").map(|s| s.parse::<usize>().expect("Invalid dimension")).collect::<Vec<_>>();
                (arr[0], arr[1])
            });
        }
        if iter_regex.is_match(m).expect("Error when looking for iterations") {
            let caps = iter_regex.captures(m).expect("Error when looking for iterations").unwrap();
            let n = caps.get(1).expect("Unable to find iterations!").as_str();
            max_iters = n.parse::<u16>().expect("Invalid iteration count");
        }
        if delay_regex.is_match(m).expect("Error when looking for delay") {
            let caps = delay_regex.captures(m).expect("Error when looking for delay").unwrap();
            let n = caps.get(1).expect("Unable to find delay!").as_str();
            delay = Some(n.parse::<u16>().expect("Invalid delay count"));
        }
        if print_regex.is_match(m).expect("Error when looking for print") {
            print = true;
        }
    }

    let dims = dims.expect("No dimensions found");

    let mut grid = vec![vec![' '; dims.0]; dims.1];
    for (y, line) in fstr.lines().filter(|l| {
        let t = l.trim();
        !t.is_empty() && t.chars().nth(0) != Some('#')
    }).enumerate() {
        for (x, c) in line.chars().enumerate() {
            grid[y][x] = c;
        }
    }

    let ins = pos_of_chars(&grid, 'i');

    println!();
    let mut inputs = HashMap::new();
    for i in ins {
        let mut input = String::new();
        println!("Enter input for cell at ({}, {}): ", i.0, i.1);
        std::io::stdin().read_line(&mut input).expect("Error reading input");
        println!();
        let num = input.trim().parse::<f64>().expect("Invalid numerical input");
        inputs.insert(i, num);
    }

    let mut cells = inputs.iter().map(|(&pos, &val)| Cell::new(pos, val, Dir::Down)).collect::<Vec<Cell>>();
    let mut iters = 0;

    while cells.len() > 0 && iters < max_iters {
        if max_iters != 0 {
            iters += 1;
        }
        let rep = cells.iter().map(|c| (c.pos, c.val)).collect::<HashMap<_, _>>();

        if print {
            print!("{}[2J", 27 as char);
            for y in 0..dims.1 {
                for x in 0..dims.0 {
                    if rep.contains_key(&(x, y)) {
                        print!("{}", format!("{:.0}", rep[&(x, y)] % 10.).red());
                    } else {
                        print!("{}", grid[y][x]);
                    }
                }
                println!();
            }
            println!();
        }
        let mut cells_to_remove = Vec::new();
        let mut cells_to_add = Vec::new();
        let len = cells.len();
        for (i, cell) in cells.iter_mut().enumerate() {
            if cell.ignore {
                continue;
            }
            let cur = *grid.get(cell.pos.1).and_then(|r| r.get(cell.pos.0)).unwrap_or(&'.');

            match cur {
                // output
                'o' => {
                    print!("{}", cell.val);
                    io::stdout().flush().unwrap();
                    cells_to_remove.push(i);
                }
                // even
                'e' => {
                    cell.dir = if cell.val % 2. == 0f64 {
                        Dir::Right
                    } else {
                        Dir::Left
                    }
                }
                '>' => cell.dir = Dir::Right,
                '<' => cell.dir = Dir::Left,
                '^' => cell.dir = Dir::Up,
                'v' => cell.dir = Dir::Down,
                // slow
                's' => {
                    if cell.frozen {
                        cell.frozen = false;
                    } else {
                        cell.frozen = true;
                    }
                }
                '.' => cells_to_remove.push(i),
                '+' => cell.val += 1f64,
                '-' => cell.val -= 1f64,
                '*' => cell.val *= 2f64,
                '/' => cell.val /= 2f64,
                // bounce
                '\\' => {
                    cell.dir = match cell.dir {
                        Dir::Up => Dir::Left,
                        Dir::Down => Dir::Right,
                        Dir::Left => Dir::Up,
                        Dir::Right => Dir::Down,
                        Dir::Neutral => Dir::Neutral,
                    }
                }
                // negate
                '!' => {
                    if cell.val == 0. {
                        cell.val = 1.;
                    } else {
                        cell.val = -cell.val;
                    }
                }
                // negative?
                '_' => conditional(cell, cell.val < 0.),
                'n' => cell.val = cell.val.clamp(0., 1.),
                'c' => cell.val = cell.val.ceil(),
                'f' => cell.val = cell.val.floor(),
                '%' => cell.val = cell.val.fract(),
                // poll
                '?' => {
                    let mut input = String::new();
                    println!("Enter input for cell at ({}, {}): ", cell.pos.0, cell.pos.1);
                    io::stdin().read_line(&mut input).expect("Error reading input");
                    println!();
                    cell.val = input.trim().parse::<f64>().expect("Invalid numerical input");
                }
                'p' | 'q' => if cell.dir == Dir::Up || cell.dir == Dir::Down {
                    cells_to_add.push(Cell::new(cell.pos, cell.val, if cur == 'p' { Dir::Right } else { Dir::Left }));
                }
                'w' | 'm' => if cell.dir == Dir::Left || cell.dir == Dir::Right {
                    cells_to_add.push(Cell::new(cell.pos, cell.val, if cur == 'w' { Dir::Down } else { Dir::Up }));
                }
                // split/jump
                '|' => {
                    if cell.dir == Dir::Up || cell.dir == Dir::Down {
                        cell.dir = Dir::Left;
                        cells_to_add.push(Cell::new(cell.pos, cell.val, Dir::Right));
                    } else {
                        match cell.dir {
                            Dir::Left => shift(cell, (-1, 0)),
                            Dir::Right => shift(cell, (1, 0)),
                            _ => {}
                        }
                    }
                }
                // split/jump
                '=' => {
                    if cell.dir == Dir::Left || cell.dir == Dir::Right {
                        cell.dir = Dir::Up;
                        cells_to_add.push(Cell::new(cell.pos, cell.val, Dir::Down));
                    } else {
                        match cell.dir {
                            Dir::Up => shift(cell, (0, -1)),
                            Dir::Down => shift(cell, (0, 1)),
                            _ => {}
                        }
                    }
                }
                'z' => {
                    conditional(cell, cell.val == 0.);
                }
                _ => {}
            }
        }
        // check for collision
        for mut c1 in 0..cells.len() {
            if cells[c1].ignore {
                continue;
            }
            for mut c2 in 0..cells.len() {
                if cells[c2].ignore {
                    continue;
                }
                if c1 == c2 {
                    continue;
                }

                if cells[c1].pos == cells[c2].pos {
                    if cells[c1].val == cells[c2].val {
                        if cells[c1].dir == Dir::Right || cells[c1].dir == Dir::Down {
                            cells[c1].val += cells[c2].val;
                            cells[c2].ignore = true;
                            cells_to_remove.push(c2);
                        } else {
                            cells[c2].val += cells[c1].val;
                            cells[c1].ignore = true;
                            cells_to_remove.push(c1);
                        }
                    } else if cells[c1].val > cells[c2].val {
                        cells[c1].val += cells[c2].val;
                        cells[c2].ignore = true;
                        cells_to_remove.push(c2);
                    } else {
                        cells[c2].val += cells[c1].val;
                        cells[c1].ignore = true;
                        cells_to_remove.push(c1);
                    }
                }
            }
        }
        for i in cells_to_remove.iter().rev() {
            if let Some(cell) = cells.get(*i) {
                cells.remove(*i);
            }
        }
        for cell in cells_to_add.iter() {
            cells.push(cell.clone());
        }
        for cell in cells.iter_mut() {
            match cell.dir {
                Dir::Neutral => {}
                _ if !cell.frozen => {
                    let dir: (i8, i8) = cell.dir.clone().into();
                    shift(cell, dir);
                }
                _ => {}
            }
        }
        if delay.is_some() {
            std::thread::sleep(std::time::Duration::from_millis(delay.unwrap() as u64));
        }
    }
}
fn pos_of_chars(grid: &Vec<Vec<char>>, c: char) -> Vec<(usize, usize)> {
    let mut pos = Vec::new();
    for (i, row) in grid.iter().enumerate() {
        for (j, ch) in row.iter().enumerate() {
            if *ch == c {
                pos.push((j, i));
            }
        }
    }
    pos
}

fn move_cell(cell: &mut Cell, to: (usize, usize)) {
    cell.pos = to;
}
fn shift(cell: &mut Cell, dir: (i8, i8)) {
    cell.pos = ((cell.pos.0 as isize + dir.0 as isize) as usize, (cell.pos.1 as isize + dir.1 as isize) as usize);
}

#[derive(Clone, Debug, PartialEq)]
struct Cell {
    pub pos: (usize, usize),
    pub val: f64,
    pub dir: Dir,
    pub frozen: bool,
    pub ignore: bool, // cell will be deleted, don't bother to update
}
impl Cell {
    pub fn new(pos: (usize, usize), val: f64, dir: Dir) -> Self {
        Self { pos, val, dir, frozen: false, ignore: false }
    }
}
#[derive(Clone, Debug, PartialEq)]
enum Dir {
    Neutral,
    Up,
    Down,
    Left,
    Right,
}
impl Into<(i8, i8)> for Dir {
    fn into(self) -> (i8, i8) {
        match self {
            Dir::Neutral => (0, 0),
            Dir::Up => (0, -1),
            Dir::Down => (0, 1),
            Dir::Left => (-1, 0),
            Dir::Right => (1, 0),
        }
    }
}
fn conditional(cell: &mut Cell, cond: bool) {
    match cell.dir {
        Dir::Up | Dir::Down => {
            if cond {
                cell.dir = Dir::Right;
            } else {
                cell.dir = Dir::Left;
            }
        }
        Dir::Left | Dir::Right => {
            if cond {
                cell.dir = Dir::Down;
            } else {
                cell.dir = Dir::Up;
            }
        }
        _ => {}
    }
}