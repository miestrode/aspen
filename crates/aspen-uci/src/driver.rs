use std::io::{StdoutLock, Write};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{Receiver, Sender, channel};
use std::thread::{Scope, spawn};

use shakmaty::{CastlingMode, Chess, Move, Position};
use shakmaty::uci::UciMove;

use crate::{Engine, PositionSpec, Response, UciCommand, UciOptions};

enum Event {
    Command(UciCommand),
    SearchFinished,
}

type SearchResult<EngineType> = (EngineType, Option<Move>);

pub fn run<EngineType: Engine>(engine: EngineType) -> std::io::Result<()> {
    let stop = AtomicBool::new(false);
    let (sender, receiver) = channel();
    spawn_input_reader(sender.clone());
    std::thread::scope(|scope| {
        let mut driver = Driver::new(engine);
        driver.run(scope, &stop, sender, receiver)
    })
}

fn spawn_input_reader(sender: Sender<Event>) {
    spawn(move || {
        for line in std::io::stdin().lines() {
            let Ok(line) = line else { break };
            match line.parse::<UciCommand>() {
                Ok(command) => {
                    if sender.send(Event::Command(command)).is_err() {
                        break;
                    }
                }
                Err(error) => eprintln!("{error}"),
            }
        }
    });
}

struct Driver<EngineType: Engine> {
    engine: Option<EngineType>,
    board: Chess,
    pending: Option<oneshot::Receiver<SearchResult<EngineType>>>,
    output: StdoutLock<'static>,
    debug: bool,
}

impl<EngineType: Engine> Driver<EngineType> {
    fn new(engine: EngineType) -> Self {
        Driver {
            engine: Some(engine),
            board: Chess::default(),
            pending: None,
            output: std::io::stdout().lock(),
            debug: false,
        }
    }

    fn run<'scope, 'environment>(
        &mut self,
        scope: &'scope Scope<'scope, 'environment>,
        stop: &'scope AtomicBool,
        sender: Sender<Event>,
        receiver: Receiver<Event>,
    ) -> std::io::Result<()>
    where
        EngineType: 'scope,
    {
        for event in receiver {
            match event {
                Event::Command(command) => {
                    if self.handle(scope, command, stop, &sender)? {
                        break;
                    }
                }
                Event::SearchFinished => self.finish_search()?,
            }
        }
        Ok(())
    }

    fn handle<'scope, 'environment>(
        &mut self,
        scope: &'scope Scope<'scope, 'environment>,
        command: UciCommand,
        stop: &'scope AtomicBool,
        sender: &Sender<Event>,
    ) -> std::io::Result<bool>
    where
        EngineType: 'scope,
    {
        match command {
            UciCommand::Uci => self.announce()?,
            UciCommand::Debug(value) => self.debug = value,
            UciCommand::IsReady => self.emit(Response::ReadyOk)?,
            UciCommand::SetOption { name, value } => self.set_option(&name, value.as_deref()),
            UciCommand::UciNewGame => self.new_game(),
            UciCommand::Position(spec) => self.resolve_position(spec),
            UciCommand::Go(params) => self.start_search(scope, params, stop, sender),
            UciCommand::Stop => stop.store(true, Ordering::Relaxed),
            UciCommand::PonderHit => {}
            UciCommand::Quit => {
                stop.store(true, Ordering::Relaxed);
                return Ok(true);
            }
        }
        Ok(false)
    }

    fn announce(&mut self) -> std::io::Result<()> {
        self.emit(Response::IdName(EngineType::NAME))?;
        self.emit(Response::IdAuthor(EngineType::AUTHOR))?;
        for declaration in EngineType::Options::declarations() {
            self.emit(Response::Option(declaration))?;
        }
        self.emit(Response::UciOk)
    }

    fn new_game(&mut self) {
        self.board = Chess::default();
        if let Some(engine) = self.engine.as_mut() {
            engine.new_game();
        }
    }

    fn set_option(&mut self, name: &str, value: Option<&str>) {
        let Some(engine) = self.engine.as_mut() else {
            return;
        };
        if let Err(error) = engine.options().set(name, value) {
            eprintln!("{error}");
        }
    }

    fn resolve_position(&mut self, spec: PositionSpec) {
        match resolve_board(spec) {
            Ok(board) => self.board = board,
            Err(error) => eprintln!("{error}"),
        }
    }

    fn start_search<'scope, 'environment>(
        &mut self,
        scope: &'scope Scope<'scope, 'environment>,
        params: crate::GoParams,
        stop: &'scope AtomicBool,
        sender: &Sender<Event>,
    ) where
        EngineType: 'scope,
    {
        let Some(mut engine) = self.engine.take() else {
            return;
        };
        stop.store(false, Ordering::Relaxed);
        let board = self.board.clone();
        let sender = sender.clone();
        let (result_sender, result_receiver) = oneshot::channel();
        self.pending = Some(result_receiver);
        scope.spawn(move || {
            let best = engine.go(&board, &params, stop);
            let _ = result_sender.send((engine, best));
            let _ = sender.send(Event::SearchFinished);
        });
    }

    fn finish_search(&mut self) -> std::io::Result<()> {
        let Some(receiver) = self.pending.take() else {
            return Ok(());
        };
        let Ok((engine, best)) = receiver.recv() else {
            return Ok(());
        };
        self.engine = Some(engine);
        let best = best
            .map(|best| best.to_uci(CastlingMode::Standard))
            .unwrap_or(UciMove::Null);
        self.emit(Response::BestMove { best, ponder: None })
    }

    fn emit(&mut self, response: Response) -> std::io::Result<()> {
        writeln!(self.output, "{response}")?;
        self.output.flush()
    }
}

fn resolve_board(spec: PositionSpec) -> Result<Chess, Box<dyn std::error::Error>> {
    let (mut board, moves) = match spec {
        PositionSpec::StartPos { moves } => (Chess::default(), moves),
        PositionSpec::Fen { fen, moves } => {
            (fen.into_position(CastlingMode::Standard)?, moves)
        }
    };
    for uci_move in moves {
        let resolved = uci_move.to_move(&board)?;
        board.play_unchecked(resolved);
    }
    Ok(board)
}
