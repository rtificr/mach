use std::collections::HashMap;
use std::env;
use std::fs::File;
use std::io::Read;
use colored::Colorize;
use fancy_regex::Regex;

const DEFAULT_MAX_ITERS: u16 = 1024;
fn main() {
    let path = env::args().nth(1).unwrap_or(String::from("program.rt.mach"));
    let mut file = File::open(path).expect("File not found");
    let mut fstr = String::new();

    file.read_to_string(&mut fstr).unwrap();
    let meta_regex = Regex::new(r"(?<=#).*(?=\n)").expect("Invalid regex");
    let metas = meta_regex.find_iter(&fstr)
        .map(|m| m.expect("Error when looking for meta").as_str()).collect::<Vec<_>>();

    let dim_regex = Regex::new(r"^\d+\*\d+$").expect("Invalid regex");
    let iter_regex = Regex::new(r"^(?:iters\s*)(\d+)$").expect("Invalid regex");
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

    let mut grid = vec![vec!['.'; dims.0]; dims.1];

    for (y, line) in fstr.lines().map(|l| l.trim()).filter(|l| !l.is_empty() && l.chars().nth(0) != Some('#')).enumerate() {
        for (x, c) in line.chars().enumerate() {
            grid[y][x] = c;
        }
    }

    let ins = pos_of_chars(&grid, 'i');

    let mut inputs = HashMap::new();
    for i in ins {
        let mut input = String::new();
        println!();
        println!("Enter input for cell at ({}, {}): ", i.0, i.1);
        std::io::stdin().read_line(&mut input).expect("Error reading input");
        println!();
        let num = input.trim().parse::<i32>().expect("Invalid numerical input");
        inputs.insert(i, num);
    }

    let mut cells = inputs.clone();
    let mut iters = 0;
    while cells.len() > 0 && iters < DEFAULT_MAX_ITERS {
        if max_iters != 0 {
            iters += 1;
        }
        if print {
            for y in 0..dims.1 {
                for x in 0..dims.0 {
                    if cells.contains_key(&(x, y)) {
                        print!("{}", format!("{}", cells[&(x,y)]).red());
                    } else {
                        print!("{}", grid[y][x]);
                    }
                }
                println!();
            }
            println!();
        }
        for (pos, val) in cells.clone().iter() {
            match grid[pos.1][pos.0] {
                'o' => {
                    print!("{val}");
                    cells.remove(pos);
                }
                'e' => {
                    if *val % 2 == 0 {
                        move_cell(&mut cells, *pos, (pos.0 + 1, pos.1));
                    } else {
                        move_cell(&mut cells, *pos, (pos.0 - 1, pos.1));
                    }
                }
                '>' => move_cell(&mut cells, *pos, (pos.0 + 1, pos.1)),
                '<' => move_cell(&mut cells, *pos, (pos.0 - 1, pos.1)),
                '^' => move_cell(&mut cells, *pos, (pos.0, pos.1 - 1)),
                'v' => move_cell(&mut cells, *pos, (pos.0, pos.1 + 1)),
                '0'..'9' => {
                    let num = grid[pos.1][pos.0].to_digit(10).expect("Invalid digit") as i32;
                    cells.insert(*pos, num);
                    move_cell(&mut cells, *pos, (pos.0, pos.1 + 1));
                }
                '.' => {
                    cells.remove(pos);
                }
                _ => {
                    move_cell(&mut cells, *pos, (pos.0, pos.1 + 1));
                }
            }
            if iters == 0 && cells.len() > 0 {
                println!("Ran out of iterations");
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
                pos.push((i, j));
            }
        }
    }
    pos
}

fn move_cell(
    cells: &mut HashMap<(usize, usize), i32>,
    from: (usize, usize),
    to: (usize, usize))
{
    if cells.contains_key(&to) {
        cells.insert(to, cells[&from] + cells[&to]);
        cells.remove(&from);
    } else {
        cells.insert(to, cells[&from]);
        cells.remove(&from);
    }
}