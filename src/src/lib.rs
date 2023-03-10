extern crate js_sys;
extern crate wasm_bindgen;
extern crate web_sys;

use js_sys::Math;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::{CanvasRenderingContext2d, FocusEvent, HtmlCanvasElement, KeyboardEvent};

use std::cell::RefCell;
use std::rc::Rc;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = console)]
    fn log(s: &str);
    #[wasm_bindgen(js_namespace = console)]
    fn error(s: &str);
    fn alert(s: &str);
}

#[derive(Debug, Copy, Clone, PartialEq)]
enum Block {
    Blank,
    Fill,
}

#[macro_use]
extern crate litcrypt;

use_litcrypt!();

use self::Block::*;

#[derive(Debug, Copy, Clone, PartialEq)]
enum TetroTypes {
    I,
    J,
    L,
    O,
    S,
    T,
    Z,
}

use self::TetroTypes::*;

// (x, y) -> (col, row)
type TetroCoords = [(i32, i32); 4];

#[derive(Debug, Copy, Clone)]
struct Tetromino {
    t: TetroTypes,
    coords: TetroCoords,
}

impl Tetromino {
    fn random(x0: i32) -> Tetromino {
        let r = Math::floor(Math::random() * 7.0) as usize;
        let t = [I, J, L, O, S, T, Z][r];
        let coords = match t {
            I => [(x0, -4), (x0, -3), (x0, -2), (x0, -1)],
            J => [(x0 + 1, -3), (x0 + 1, -2), (x0 + 1, -1), (x0, -1)],
            L => [(x0, -3), (x0, -2), (x0, -1), (x0 + 1, -1)],
            O => [(x0, -2), (x0 + 1, -2), (x0, -1), (x0 + 1, -1)],
            S => [(x0 + 2, -2), (x0 + 1, -2), (x0 + 1, -1), (x0, -1)],
            T => [(x0, -2), (x0 + 1, -2), (x0 + 2, -2), (x0 + 1, -1)],
            Z => [(x0, -2), (x0 + 1, -2), (x0 + 1, -1), (x0 + 2, -1)],
        };
        Tetromino { t, coords }
    }
}

fn derived_level(score: u32) -> u32 {
    match score {
        0..=999 => 1,
        1_000..=2_999 => 2,
        3_000..=4_999 => 3,
        5_000..=6_999 => 4,
        _ => 5,
    }
}

fn derived_speed(level: u32) -> f64 {
    match level {
        0 | 1 => 300.0,
        2 => 200.0,
        3 => 100.0,
        4 => 50.0,
        _ => 20.0,
    }
}

struct Core {
    rows: usize,
    cols: usize,
    block_width: u32,
    matrix: Vec<Vec<Block>>,
    current_tetro: Tetromino,
    next_tetro: Tetromino,
    score: u32,
    level: u32,
    speed: f64,
    game_over: bool,
    game_win: bool,
}

impl Core {
    fn new(rows: usize, cols: usize, block_width: u32) -> Core {
        let x0 = cols as i32 / 2 - 1;
        let current_tetro = Tetromino::random(x0);
        let next_tetro = Tetromino::random(x0);

        Core {
            rows,
            cols,
            block_width,
            matrix: vec![vec![Blank; cols]; rows],
            current_tetro,
            next_tetro,
            score: 0,
            level: 1,
            speed: 300.0,
            game_over: false,
            game_win: false,
        }
    }
}

impl Core {
    fn will_crash(&self, new_coords: TetroCoords) -> bool {
        new_coords.iter().any(|&(x, y)| {
            x < 0
                || x >= self.cols as i32
                || y >= self.rows as i32
                || (x >= 0 && y >= 0 && self.matrix[y as usize][x as usize] == Fill)
        })
    }
}

impl Core {
    fn fill_in(&mut self) {
        for &(x, y) in &self.current_tetro.coords {
            if self.score > 100000 {
                self.game_win = true;
            }
            if y < 0 {
                self.game_over = true;
            } else {
                self.matrix[y as usize][x as usize] = Fill;
            }
        }

        self.current_tetro = self.next_tetro.clone();
        self.next_tetro = Tetromino::random(self.cols as i32 / 2 - 1);
        self.matrix.retain(|row| row.iter().any(|&b| b == Blank));

        let rows_remain = self.matrix.len();
        if rows_remain < self.rows {
            let rows_missing = self.rows - rows_remain;
            let mut new_matrix = vec![vec![Blank; self.cols]; rows_missing];
            new_matrix.append(&mut self.matrix);
            self.matrix = new_matrix;
            self.score += 100 * 2u32.pow(rows_missing as u32 - 1);
            self.level = derived_level(self.score);
            self.speed = derived_speed(self.level);
        }
    }
}

impl Core {
    fn move_down(&mut self) -> bool {
        let mut new_coords = self.current_tetro.coords.clone();
        for c in new_coords.iter_mut() {
            c.1 += 1;
        }
        if self.will_crash(new_coords) {
            self.fill_in();
            false
        } else {
            self.current_tetro.coords = new_coords;
            true
        }
    }
}

impl Core {
    fn drop_down(&mut self) {
        while self.move_down() {}
    }
}

impl Core {
    fn move_left(&mut self) {
        let mut new_coords = self.current_tetro.coords.clone();
        for c in new_coords.iter_mut() {
            c.0 -= 1;
        }
        if !self.will_crash(new_coords) {
            self.current_tetro.coords = new_coords;
        }
    }
}

impl Core {
    fn move_right(&mut self) {
        let mut new_coords = self.current_tetro.coords.clone();
        for c in new_coords.iter_mut() {
            c.0 += 1;
        }
        if !self.will_crash(new_coords) {
            self.current_tetro.coords = new_coords;
        }
    }
}

impl Core {
    fn rotate(&mut self) {
        if self.current_tetro.t == O {
            return;
        }
        // use `dx` to adjust origin horizontally
        for dx in [0, -1, 1, -2, 2].iter() {
            let mut new_coords = self.current_tetro.coords.clone();
            // rotate origin
            let (mut x0, y0) = new_coords[1];
            x0 += dx;
            // rotate 90 degree
            // https://en.wikipedia.org/wiki/Rotation_of_axes
            for c in new_coords.iter_mut() {
                c.0 += dx;
                *c = (x0 + y0 - c.1, y0 + c.0 - x0);
            }
            if !self.will_crash(new_coords) {
                self.current_tetro.coords = new_coords;
                break;
            }
        }
    }
}

impl Core {
    fn restart(&mut self) {
        let x0 = self.cols as i32 / 2 - 1;
        self.current_tetro = Tetromino::random(x0);
        self.next_tetro = Tetromino::random(x0);
        self.matrix = vec![vec![Blank; self.cols]; self.rows];
        self.score = 0;
        self.game_over = false;
        self.game_win = false;
        self.speed = 300.0;
    }
}

struct Tetris {
    core: Core,
    context: CanvasRenderingContext2d,
    width: u32,
    height: u32,
    delta: u32,
    header_height: u32,
    color_blank: JsValue,
    color_fill: JsValue,
}

impl Tetris {
    fn make(
        canvas: &HtmlCanvasElement,
        rows: usize,
        cols: usize,
        block_width: u32,
    ) -> Rc<RefCell<Tetris>> {
        if rows < 12 || cols < 10 || block_width < 10 {
            let required = "Required: rows >= 12 && cols >= 10 && block_width >= 10";
            error(required);
            panic!("{}", required);
        }

        let core = Core::new(rows, cols, block_width);
        let delta = block_width + 1;
        let header_height = 60u32;
        let width = cols as u32 * delta;
        let height = header_height + rows as u32 * delta;
        let color_blank = JsValue::from_str("#eee");
        let color_fill = JsValue::from_str("#333");
        let context = canvas
            .get_context("2d")
            .unwrap()
            .unwrap()
            .dyn_into::<CanvasRenderingContext2d>()
            .unwrap();

        canvas.set_width(width);
        canvas.set_height(height);
        canvas.set_attribute("tabindex", "0").unwrap();
        context.set_text_align("center");
        context.set_fill_style(&color_blank);

        let rc_tetris = Rc::new(RefCell::new(Tetris {
            core,
            context,
            width,
            height,
            delta,
            header_height,
            color_blank,
            color_fill,
        }));

        setup(canvas, rc_tetris.clone());
        rc_tetris.borrow_mut().render();
        rc_tetris
    }
}

impl Tetris {
    fn render(&mut self) {
        let x_center = self.width as f64 / 2.0;
        let score_level = String::from("SCORE/LEVEL: ")
            + &self.core.score.to_string()
            + "/"
            + &self.core.level.to_string();
        let block_width = self.core.block_width as f64;
        let header_height = self.header_height as f64;
        let delta = self.delta as f64;

        let game_flag = lc!("aabbccflagccbbaa");

        self.context
            .clear_rect(0.0, 0.0, self.width as f64, self.height as f64);
        self.context.set_fill_style(&self.color_fill);
        self.context.set_font("12px sans-serif");
        self.context
            .fill_text(&score_level, x_center, 10.0)
            .unwrap();

        if self.core.game_win {
            self.context.set_font("18px sans-serif");
            let answer = game_flag;
            self.core.game_over = true;
            alert(&format!("Congratulations~ \n{}!", &answer));
        } else {
            if self.core.game_over {
                self.context.set_font("18px sans-serif");
                self.context
                    .fill_text("Game Over!", x_center, 35.0)
                    .unwrap();
            } else {
                // next tetro
                let w = 8.0;
                let delta = w + 1.0;
                let x0 = (self.core.cols / 2) as i32;
                for (x, y) in self.core.next_tetro.coords.iter() {
                    self.context.fill_rect(
                        (*x - x0) as f64 * delta + x_center,
                        header_height + *y as f64 * delta,
                        w,
                        w,
                    );
                }
            }
        }

        for row in 0..self.core.rows {
            for col in 0..self.core.cols {
                self.context
                    .set_fill_style(if self.core.matrix[row][col] == Fill {
                        &self.color_fill
                    } else {
                        &self.color_blank
                    });
                self.context.fill_rect(
                    col as f64 * delta,
                    header_height + row as f64 * delta,
                    block_width,
                    block_width,
                );
            }
        }
        self.context.set_fill_style(&self.color_fill);
        for (x, y) in self.core.current_tetro.coords.iter() {
            if y >= &0 {
                self.context.fill_rect(
                    *x as f64 * delta,
                    header_height + *y as f64 * delta,
                    block_width,
                    block_width,
                );
            }
        }
    }
}

// https://github.com/rustwasm/wasm-bindgen/blob/3d2f548ce2/examples/request-animation-frame/src/lib.rs
fn setup(canvas: &HtmlCanvasElement, rc_tetris: Rc<RefCell<Tetris>>) {
    let id = Rc::new(RefCell::new(None));
    let id1 = id.clone();
    let id2 = id.clone();
    let f = Rc::new(RefCell::new(None));
    let f1 = f.clone();
    let f2 = f.clone();
    let t1 = rc_tetris.clone();
    let mut time_stamp = 0.0;

    *f1.borrow_mut() = Some(Closure::wrap(Box::new(move |time: f64| {
        if time - time_stamp > rc_tetris.borrow().core.speed {
            time_stamp = time;
            let mut t = rc_tetris.borrow_mut();
            if !t.core.game_over {
                t.core.move_down();
                t.render();
            }
        }
        *id.borrow_mut() = Some(request_animation_frame(f.borrow().as_ref().unwrap()));
    }) as Box<dyn FnMut(_)>));

    let focus_handler = Closure::wrap(Box::new(move |e: FocusEvent| {
        if e.type_() == "focus" {
            request_animation_frame(f1.borrow().as_ref().unwrap());
        } else {
            if let Some(i) = *id1.borrow() {
                cancel_animation_frame(i);
            }
            *id1.borrow_mut() = None;
        }
    }) as Box<dyn FnMut(_)>);

    let keyboard_handler = Closure::wrap(Box::new(move |e: KeyboardEvent| {
        let mut t = t1.borrow_mut();
        let playing = if let Some(_) = *id2.borrow() {
            true
        } else {
            false
        };
        if t.core.game_over {
            match e.key().as_str() {
                "r" | "Enter" => {
                    t.core.restart();
                    t.render();
                }
                _ => (),
            }
            return;
        }
        if playing {
            match e.key().as_str() {
                "ArrowUp" | "w" | "i" => t.core.rotate(),
                "ArrowRight" | "d" | "l" => t.core.move_right(),
                "ArrowLeft" | "a" | "j" => t.core.move_left(),
                "ArrowDown" | "s" | "k" => {
                    t.core.move_down();
                }
                "Enter" | " " => {
                    e.prevent_default();
                    t.core.drop_down();
                }
                "p" => {
                    if let Some(i) = *id2.borrow() {
                        cancel_animation_frame(i);
                    }
                    *id2.borrow_mut() = None;
                }
                "r" => t.core.restart(),
                _ => (),
            }
        } else {
            request_animation_frame(f2.borrow().as_ref().unwrap());
        }
        t.render();
    }) as Box<dyn FnMut(_)>);

    canvas
        .add_event_listener_with_event_listener("blur", focus_handler.as_ref().unchecked_ref())
        .unwrap();
    canvas
        .add_event_listener_with_event_listener("focus", focus_handler.as_ref().unchecked_ref())
        .unwrap();
    canvas
        .add_event_listener_with_event_listener(
            "keydown",
            keyboard_handler.as_ref().unchecked_ref(),
        )
        .unwrap();

    focus_handler.forget();
    keyboard_handler.forget();
}

fn window() -> web_sys::Window {
    web_sys::window().expect("global `window` should be OK.")
}

fn request_animation_frame(f: &Closure<dyn FnMut(f64)>) -> i32 {
    window()
        .request_animation_frame(f.as_ref().unchecked_ref())
        .expect("`requestAnimationFrame` should be OK.")
}

fn cancel_animation_frame(id: i32) {
    window()
        .cancel_animation_frame(id)
        .expect("`cancelAnimationFrame` should be OK.");
}

#[wasm_bindgen]
pub fn make_tetris(rows: usize, cols: usize, block_width: u32) -> HtmlCanvasElement {
    let canvas = window()
        .document()
        .unwrap()
        .create_element("canvas")
        .unwrap()
        .dyn_into::<HtmlCanvasElement>()
        .unwrap();
    Tetris::make(&canvas, rows, cols, block_width);
    canvas
}
