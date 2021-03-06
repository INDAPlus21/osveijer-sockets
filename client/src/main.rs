use ggez::{conf, event, graphics, ContextBuilder, Context, GameError, GameResult};
use std::path;
use osveijer_chess::{Game, Colour, Piece};
use std::io::{ErrorKind, Read, Write};
use std::net::TcpStream;
use std::sync::mpsc::{self, TryRecvError, Sender, Receiver};
use std::thread;
use std::time::Duration;

/* Address to server. */
const SERVER_ADDR: &str = "127.0.0.1:6000";

/* Max message size in characters. */
const MSG_SIZE: usize = 32;

/// A chess board is 8x8 tiles.
const GRID_SIZE: i16 = 8;
/// Sutible size of each tile.
const GRID_CELL_SIZE: (i16, i16) = (90, 90);

/// Size of the application window.
const SCREEN_SIZE: (f32, f32) = (
    GRID_SIZE as f32 * GRID_CELL_SIZE.0 as f32,
    GRID_SIZE as f32 * GRID_CELL_SIZE.1 as f32,
);

// GUI Color representations
const BLACK: graphics::Color = graphics::Color::new(228.0/255.0, 196.0/255.0, 108.0/255.0, 1.0);
const WHITE: graphics::Color = graphics::Color::new(188.0/255.0, 140.0/255.0, 76.0/255.0, 1.0);
const SELECTED: graphics::Color = graphics::Color::new(0.0, 140.0/255.0, 10.0/255.0, 0.8);
const HIGHLIGHTED: graphics::Color = graphics::Color::new(0.0, 140.0/255.0, 10.0/255.0, 0.3);

/// GUI logic and event implementation structure. 
struct AppState {
    sprites: Vec<(Piece, graphics::Image)>,
    game: Game,
    // Save piece positions, which tiles has been clicked, current colour, etc...
    selected_square: Option<(usize,usize)>,
    highlighted_squares: Vec<(usize,usize)>,
    sender: Sender<String>,
    receiver: Receiver<String> 
}

impl AppState {
    /// Initialise new application, i.e. initialise new game and load resources.
    fn new(ctx: &mut Context, _sender: Sender<String>, _receiver: Receiver<String>) -> GameResult<AppState> {

        
        let state = AppState {
            sprites: AppState::load_sprites(ctx),
            game: Game::new(),
            selected_square: None,
            highlighted_squares: vec![],
            sender: _sender,
            receiver: _receiver
        };

        Ok(state)
    }

    /// Loads chess piese images into vector.
    fn load_sprites(ctx: &mut Context) -> Vec<(Piece, graphics::Image)> {

        [
            (Piece::King(Colour::Black), "/black_king.png".to_string()),
            (Piece::Queen(Colour::Black), "/black_queen.png".to_string()),
            (Piece::Rook(Colour::Black), "/black_rook.png".to_string()),
            (Piece::Pawn(Colour::Black), "/black_pawn.png".to_string()),
            (Piece::Bishop(Colour::Black), "/black_bishop.png".to_string()),
            (Piece::Knight(Colour::Black), "/black_knight.png".to_string()),
            (Piece::King(Colour::White), "/white_king.png".to_string()),
            (Piece::Queen(Colour::White), "/white_queen.png".to_string()),
            (Piece::Rook(Colour::White), "/white_rook.png".to_string()),
            (Piece::Pawn(Colour::White), "/white_pawn.png".to_string()),
            (Piece::Bishop(Colour::White), "/white_bishop.png".to_string()),
            (Piece::Knight(Colour::White), "/white_knight.png".to_string())
        ]
        .iter()
        .map(|(_piece, _path)| (*_piece, graphics::Image::new(ctx, _path).unwrap()))
        .collect::<Vec<(Piece, graphics::Image)>>()
    }
}

impl event::EventHandler<GameError> for AppState {

    /// For updating game logic, which front-end doesn't handle.
    fn update(&mut self, _ctx: &mut Context) -> GameResult {
        /* Send message in channel to server. */
        match self.receiver.try_recv() {
            // received message from channel
            Ok(msg) => {
                let positions: Vec<&str> = msg.split(" ").collect();
                self.game.make_move(positions[0].to_string(), positions[1].to_string());
                match self.selected_square {
                    Some(sel) => self.highlighted_squares = pos_coord_vec(self.game.get_possible_moves(pos_string(sel))),
                    None => ()
                }
                println!("{:?}", self.game.get_game_state());
                
            }, 
            // no message in channel
            Err(TryRecvError::Empty) => (),
            // channel has been disconnected (main thread has terminated)
            Err(TryRecvError::Disconnected) => ()
        }
        Ok(())
    }

    /// Draw interface, i.e. draw game board
    fn draw(&mut self, ctx: &mut Context) -> GameResult {

        // clear interface with gray background colour
        graphics::clear(ctx, [0.5, 0.5, 0.5, 1.0].into());


        // draw grid
        for _row in 0..8 {
            for _col in 0..8 {

                // draw tile
                let rectangle = graphics::Mesh::new_rectangle(ctx, 
                    graphics::DrawMode::fill(), 
                    graphics::Rect::new_i32(
                        _col * GRID_CELL_SIZE.0 as i32,
                        _row * GRID_CELL_SIZE.1 as i32,
                        GRID_CELL_SIZE.0 as i32,
                        GRID_CELL_SIZE.1 as i32,
                    ), match _col % 2 {
                        0 => 
                            if _row % 2 == 0 { WHITE } 
                            else { BLACK },
                        _ => 
                            if _row % 2 == 0 { BLACK } 
                            else { WHITE },
                    }).expect("Failed to create tile.");
                graphics::draw(ctx, &rectangle, graphics::DrawParam::default()).expect("Failed to draw tiles.");

                // draw piece
                if self.game.board[_row as usize][_col as usize] != None {
                    graphics::draw(ctx, &self.sprites.get(self.sprites.iter().position(|p| Some(p.0) == self.game.board[_row as usize][_col as usize]).unwrap()).unwrap().1, graphics::DrawParam::default()
                        .scale([2.0, 2.0])  // Tile size is 90 pixels, while image sizes are 45 pixels.
                        .dest(
                            [_col as f32 * GRID_CELL_SIZE.0 as f32, _row as f32 * GRID_CELL_SIZE.1 as f32],
                        )
                    ).expect("Failed to draw piece.");
                }
            }
        }

        if let Some(s) = self.selected_square {
            // draw selected square
            let rectangle = graphics::Mesh::new_rectangle(ctx, 
                graphics::DrawMode::fill(), 
                graphics::Rect::new_i32(
                    s.1 as i32 * GRID_CELL_SIZE.0 as i32,
                    s.0 as i32 * GRID_CELL_SIZE.1 as i32,
                    GRID_CELL_SIZE.0 as i32,
                    GRID_CELL_SIZE.1 as i32,
                ), 
                SELECTED
                ).expect("Failed to create tile.");
            graphics::draw(ctx, &rectangle, graphics::DrawParam::default()).expect("Failed to draw tiles.");

            // draw highlighted squares
            for squ in self.highlighted_squares.iter() {
                let rectangle = graphics::Mesh::new_rectangle(ctx, 
                    graphics::DrawMode::fill(), 
                    graphics::Rect::new_i32(
                        squ.1 as i32 * GRID_CELL_SIZE.0 as i32,
                        squ.0 as i32 * GRID_CELL_SIZE.1 as i32,
                        GRID_CELL_SIZE.0 as i32,
                        GRID_CELL_SIZE.1 as i32,
                    ), 
                    HIGHLIGHTED
                    ).expect("Failed to create tile.");
                graphics::draw(ctx, &rectangle, graphics::DrawParam::default()).expect("Failed to draw tiles.");
            }
        }
        
        // render updated graphics
        graphics::present(ctx).expect("Failed to update graphics.");

        Ok(())
    }

    /// Update game on mouse click
    fn mouse_button_up_event(&mut self, ctx: &mut Context, button: event::MouseButton, x: f32, y: f32) {
        if button == event::MouseButton::Left {
            /* check click position and update board accordingly */
            let rank = (y / GRID_CELL_SIZE.1 as f32).floor() as usize;
            let file = (x / GRID_CELL_SIZE.0 as f32).floor() as usize;
            match self.selected_square {
                Some(pos) => {
                    if pos == (rank, file) {
                        self.selected_square = None;
                        self.highlighted_squares = vec![];
                    } else if self.highlighted_squares.iter().any(|p| p == &(rank,file)) {
                        let msg: String = pos_string(pos) + " " + &pos_string((rank,file));
                        if self.sender.send(msg).is_err() {
                            println!("crashed");
                            std::process::exit(1);
                        }
                        self.selected_square = None;
                        self.highlighted_squares = vec![];
                    } else {
                        self.selected_square = Some((rank, file));
                        self.highlighted_squares = vec![];
                        let c = get_colour(self.game.board[rank][file]);
                        if c != None {
                            if c.unwrap() == self.game.active {
                                self.highlighted_squares = pos_coord_vec(self.game.get_possible_moves(pos_string((rank,file))));
                            };
                        };
                    }
                },
                None => {
                    self.selected_square = Some((rank, file));
                    self.highlighted_squares = Vec::new();
                    let c = get_colour(self.game.board[rank][file]);
                    if c != None {
                        if c.unwrap() == self.game.active {
                            self.highlighted_squares = pos_coord_vec(self.game.get_possible_moves(pos_string((rank,file))));
                        };
                    };
                }
            }
        }
    }

    fn key_down_event(
        &mut self,
        ctx: &mut Context,
        keycode: event::KeyCode,
        _keymods: event::KeyMods,
        _repeat: bool,
    ) {
        if keycode == event::KeyCode::Escape {
            event::quit(ctx);
        }
    }
}

pub fn main() -> GameResult {

    // connect to server
    let mut client = match TcpStream::connect(SERVER_ADDR) {
        Ok(_client) => {
            println!("Connected to server at: {}", SERVER_ADDR);
            _client
        },
        Err(_) => {
            println!("Failed to connect to server at: {}", SERVER_ADDR);
            std::process::exit(1)
        }
    };

    // prevent io stream operation from blocking socket in case of slow communication
    client.set_nonblocking(true).expect("Failed to initiate non-blocking!");

    // create channel for communication between threads
    let (sender, receiver) = mpsc::channel::<String>();
    let (send, rec) = mpsc::channel::<String>();

    let resource_dir = path::PathBuf::from("./resources");

    let context_builder = ContextBuilder::new("schack", "oliver")
        .add_resource_path(resource_dir)        // Import image files to GGEZ
        .window_setup(
            conf::WindowSetup::default()  
                .title("Schack")                // Set window title "Schack"
                .icon("/icon.png")              // Set application icon
        )
        .window_mode(
            conf::WindowMode::default()
                .dimensions(SCREEN_SIZE.0, SCREEN_SIZE.1) // Set window dimensions
                .resizable(false)               // Fixate window size
        );
    let (mut contex, mut event_loop) = context_builder.build().expect("Failed to build context.");

    let mut state = AppState::new(&mut contex, sender, rec).expect("Failed to create state.");
    
    /* Start thread that listens to server. */
    thread::spawn(move || loop {
        let mut msg_buffer = vec![0; MSG_SIZE];

        /* Read message from server. */
        match client.read_exact(&mut msg_buffer) {
            // received message
            Ok(_) => {
                // read until end-of-message (zero character)
                let _msg = msg_buffer
                    .into_iter()
                    .take_while(|&x| x != 0)
                    .collect::<Vec<_>>();
                let msg = String::from_utf8(_msg).expect("Invalid UTF-8 message!");
                if send.send(msg).is_err() {
                    println!("crashed");
                    std::process::exit(1);
                }
            },
            // no message in stream
            Err(ref err) if err.kind() == ErrorKind::WouldBlock => (),
            // connection error
            Err(_) => {
                println!("Lost connection with server!");
                break;
            }
        }

        /* Send message in channel to server. */
        match receiver.try_recv() {
            // received message from channel
            Ok(msg) => {
                let mut msg_buffer = msg.clone().into_bytes();
                // add zero character to mark end of message
                msg_buffer.resize(MSG_SIZE, 0);

                if client.write_all(&msg_buffer).is_err() {
                    println!("Failed to send message!")
                }
            }, 
            // no message in channel
            Err(TryRecvError::Empty) => (),
            // channel has been disconnected (main thread has terminated)
            Err(TryRecvError::Disconnected) => break
        }

        thread::sleep(Duration::from_millis(100));
    });

    event::run(contex, event_loop, state)       // Run window event loop
}

fn get_colour(piece: Option<Piece>) -> Option<Colour> {
    match piece {
        Some(p) => Some(p.unwrap()),
        None => None
    }
}

fn pos_string(_pos: (usize, usize)) -> String  {
    let mut string1 = String::new();

    string1.push(match _pos.1 {
        0 => 'a',
        1 => 'b',
        2 => 'c',
        3 => 'd',
        4 => 'e',
        5 => 'f',
        6 => 'g',
        7 => 'h',
        _ => panic!("file wrong")
    });
    match _pos.0 {
        0..=7 => string1.push(char::from_digit(_pos.0 as u32 + 1, 10).unwrap()),
        _ => panic!("rank wrong"),
    };

    string1
}

fn pos_coord_vec(vec: Option<Vec<String>>) -> Vec<(usize,usize)> {
    match vec {
        Some(v) => {
            let mut out = Vec::new();
            for i in v {
                let chars: Vec<char> = i.chars().collect();
                out.push((
                    match chars[1] {
                        '1' => 0,
                        '2' => 1,
                        '3' => 2,
                        '4' => 3,
                        '5' => 4,
                        '6' => 5,
                        '7' => 6,
                        '8' => 7,
                        _ => panic!("Rank wrong")
                    },
                    match chars[0] {
                        'a' => 0,
                        'b' => 1,
                        'c' => 2,
                        'd' => 3,
                        'e' => 4,
                        'f' => 5,
                        'g' => 6,
                        'h' => 7,
                        _ => panic!("File wrong")
                    }
                ));
            }
            out
        },
        None => vec![]
    }
}