#![no_main]
#![no_std]

use cortex_m_rt::entry;
use microbit::{
    board::Board,
    display::blocking::Display,
    hal::{
        timer::Timer,
        pac::twim0::frequency::FREQUENCY_A,
        prelude::*,
        twim::Twim,
        uarte,
        uarte::{Baudrate, Parity},
    },
};
use panic_rtt_target as _;
use rtt_target::{rprintln, rtt_init_print};
use core::fmt::Write;
use heapless::{Vec};

mod serial_setup;
use serial_setup::UartePort;

const SCREEN_SIZE: usize = 5;
const CLEAN_DISPLAY: [[u8; 5]; 5] = [
        [0, 0, 0, 0, 0],
        [0, 0, 0, 0, 0],
        [0, 0, 0, 0, 0],
        [0, 0, 0, 0, 0],
        [0, 0, 0, 0, 0],
];

/// Direction, clockwise
#[derive(Debug, Clone, Copy)]
enum Direction {
    UP,
    RIGHT,
    DOWN,
    LEFT,
}

impl Direction {
    fn right(&self) -> Self {
        match self {
            Self::UP => Self::RIGHT,
            Self::RIGHT => Self::DOWN,
            Self::DOWN => Self::LEFT,
            Self::LEFT => Self::UP,
        }
    }

    fn left(&self) -> Self {
        match self {
            Self::UP => Self::LEFT,
            Self::RIGHT => Self::UP,
            Self::DOWN => Self::RIGHT,
            Self::LEFT => Self::DOWN,
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
struct Position {
    x: usize,
    y: usize,
}

struct Food {
    position: Position
}

impl Food {
    fn regenerate(&mut self) {
        // TODO rand this
        self.position = Position { x: 3, y: 3 };
    }
}

#[derive(Debug, Clone)]
struct Snake {
    body: Vec<Position, 32>,
    direction: Direction,
}

impl Snake {
    fn new(start: Position, direction: Direction) -> Self {
        let mut body: Vec<Position, 32> = Vec::new();
        body.push(start).unwrap();
        Self { body, direction }
    }

    fn update(&mut self, button_a: bool, button_b: bool, food: &mut Food) {
        // update direction
        if button_a {
            self.direction = self.direction.left();
        }

        if button_b {
            self.direction = self.direction.right();
        }

        // update snake
        let mut new_head = self.body[0].clone();
        match self.direction {
            // some bound checking
            Direction::DOWN => new_head.y = (self.body[0].y + 1) % SCREEN_SIZE,
            Direction::RIGHT => new_head.x = (self.body[0].x + 1) % SCREEN_SIZE,
            Direction::UP => {
                let next = (self.body[0].x, (self.body[0].y as isize) - 1);
                if next.1 < 0 {
                    new_head.y = 4;
                } else {
                    new_head.y -= 1;
                }
            }
            Direction::LEFT => {
                let next = ((self.body[0].x as isize) - 1, self.body[0].y);
                if next.0 < 0 {
                    new_head.x = 4;
                } else {
                    new_head.x -= 1;
                }
            }
        };

        // add new_head
        let mut new_body: Vec<Position, 32> = Vec::new();
        new_body.push(new_head);
        self.body
            .clone()
            .into_iter()
            .map(|body_part| new_body.push(body_part));
        self.body = new_body;

        // remove tail element
        if self.body[0] != food.position {
            self.body.pop();
            food.regenerate();
        }
    }
}

#[entry]
fn main() -> ! {
    rtt_init_print!();

    let board = Board::take().unwrap();
    let mut timer = Timer::new(board.TIMER0);
    let mut display = Display::new(board.display_pins);
    let _i2c = { Twim::new(board.TWIM0, board.i2c_internal.into(), FREQUENCY_A::K100) };
    let mut uart = {
        let serial = uarte::Uarte::new(
            board.UARTE0,
            board.uart.into(),
            Parity::EXCLUDED,
            Baudrate::BAUD115200,
        );
        UartePort::new(serial)
    };

    let button_a = board.buttons.button_a;
    let button_b = board.buttons.button_b;

    let mut snake = Snake::new(Position { x: 1, y: 1 }, Direction::RIGHT);
    let mut food = Food { position: Position { x: 2, y: 2 }};

    loop {
        // update snake
        let is_a_pressed = button_a.is_low().unwrap();
        let is_b_pressed = button_b.is_low().unwrap();
        snake.update(is_a_pressed, is_b_pressed, &mut food);

        // print some debug to uart
        writeln!(uart, "Direction: {:?}\r", snake.direction);

        // update screen
        let mut display_matrix = CLEAN_DISPLAY;
        for element in snake.body.iter() {
            writeln!(uart, "Element: {:?}\r", element);
            display_matrix[element.y][element.x] = 1;
        }
        display_matrix[food.position.y][food.position.x] = 1;

        display.show(&mut timer, display_matrix, 200);
    }
}
