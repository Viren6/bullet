use std::process::{Child, Command, Output, Stdio};

use super::testing::TimeControl;

pub struct CuteChessArgs {
    pub cutechess_path: String,
    pub dev_engine_path: String,
    pub base_engine_path: String,
    pub dev_options: Vec<String>,
    pub base_options: Vec<String>,
    pub time_control: TimeControl,
    pub opening_book: String,
    pub is_pgn: bool,
    pub num_game_pairs: usize,
    pub concurrency: usize,
}

pub struct CuteChessCommand(Command);

impl CuteChessCommand {
    pub fn health_check(path: &str) -> Output {
        Command::new(path).arg("--version").output().expect("Could not start cutechess!")
    }

    fn new(path: &str) -> Self {
        Self(Command::new(path))
    }

    fn add_engine(mut self, engine_path: &str, engine_options: &[String]) -> Self {
        self.0.arg("-engine").arg(format!("cmd={engine_path}"));

        for s in engine_options {
            self.0.arg(s);
        }

        self
    }

    fn with_tc(mut self, tc: TimeControl) -> Self {
        self.0.args(["-each", "proto=uci", "timemargin=20"]);

        match tc {
            TimeControl::FixedNodes(nodes) => {
                self.0.arg("tc=inf").arg(format!("nodes={nodes}"));
            }
            TimeControl::Increment { time, inc } => {
                self.0.arg(format!("tc={time}+{inc}"));
            }
        }

        self
    }

    fn num_game_pairs(mut self, pairs: usize) -> Self {
        self.0.args(["-games", "2"]).arg("-rounds").arg(pairs.to_string()).args(["-repeat", "2"]);

        self
    }

    fn with_opening_book(mut self, book: String, is_pgn: bool) -> Self {
        self.0.args(["-openings", "policy=round", "order=random"]).arg(format!("file={book}"));

        if is_pgn {
            self.0.arg("format=pgn");
        } else {
            self.0.arg("format=epd");
        }

        self
    }

    fn with_adjudication(mut self) -> Self {
        self.0.args(["-resign", "movecount=3", "score=400", "twosided=true"]);
        self.0.args(["-draw", "movenumber=40", "movecount=8", "score=10"]);

        self
    }

    fn with_concurrency(mut self, concurrency: usize) -> Self {
        self.0.arg("-concurrency");
        self.0.arg(concurrency.to_string());

        self
    }

    fn set_stdout<T: Into<Stdio>>(mut self, out: T) -> Self {
        self.0.stdout(out);

        self
    }

    fn execute(mut self) -> Child {
        self.0.spawn().expect("Couldn't launch cutechess games!")
    }
}

pub fn run_games(args: CuteChessArgs) -> Result<(f32, f32), String> {
    let output = CuteChessCommand::new(args.cutechess_path.as_str())
        .add_engine(args.dev_engine_path.as_str(), &args.dev_options)
        .add_engine(args.base_engine_path.as_str(), &args.base_options)
        .with_tc(args.time_control)
        .num_game_pairs(args.num_game_pairs)
        .with_opening_book(args.opening_book, args.is_pgn)
        .with_adjudication()
        .with_concurrency(args.concurrency)
        .set_stdout(Stdio::piped())
        .execute();

    let output = output.wait_with_output().expect("Couldn't wait on output!");

    let stdout = String::from_utf8(output.stdout).expect("Couldn't parse stdout!");

    let mut split = stdout.split("Elo difference: ");

    let line = split.nth(1).unwrap();

    let mut split_line = line.split(',');
    let elo_segment = split_line.next().unwrap().split_whitespace().collect::<Vec<_>>();

    if let [elo, "+/-", err] = elo_segment[..] {
        Ok((elo.parse().unwrap(), err.parse().unwrap()))
    } else {
        Err(String::from("Couldn't find elo in output!"))
    }
}
