use std::{sync::mpsc::Receiver, sync::mpsc::Sender};

use anyhow::{anyhow, Result};
use cursive::{
    align::HAlign,
    event::EventResult,
    theme::BaseColor,
    traits::{Nameable as _, Resizable as _},
    utils::markup::StyledString,
    view::{Scrollable, View as _},
    views::{Dialog, FixedLayout, Layer, OnEventView, OnLayoutView, SelectView, TextView},
    Cursive, CursiveRunnable, {Rect, Vec2},
};
use memedit::*;
use procfs::process::MMapPath;
use procfs::process::Process;

const VERSION: Option<&str> = option_env!("CARGO_PKG_VERSION");
pub struct State {
    pid: Option<i32>,
    step: Step,
    sndToUi: Sender<RenderEvent>,
    rcvFromUi: Receiver<UiEvent>,
    // mapname, range, memdump, addresses of interest
}
impl State {
    pub fn run(&mut self, initial_processes: &[ProcessRef]) -> Result<()> {
        self.pid = match self.rcvFromUi.recv()? {
            UiEvent::Select(index) => Some(initial_processes[index].pid),
            _ => {
                anyhow::bail!("Failed to set PID");
            }
        };

        eprintln!("{}", self.pid.unwrap());

        Ok(())
    }
}

pub enum Step {
    GetPid,
    GetMmap,
    LoadMem,
    SearchMem, // Add search parameters - predicate (changed, unchanged, equal, <, >), data type (int, float, string)
    WriteMem,  // Add write parameters - address, length, infer data type, endianness?
}

pub enum UiEvent {
    Select(usize),
}
pub enum RenderEvent {
    MainScreen,
}

pub struct Ui {
    ui: CursiveRunnable,
}

impl Ui {
    pub fn initiate_ui(&mut self) {
        self.ui.add_global_callback('q', Cursive::quit);

        self.ui.screen_mut().add_transparent_layer(
            OnLayoutView::new(
                FixedLayout::new().child(
                    Rect::from_point(Vec2::zero()),
                    Layer::new(
                        TextView::new(format!(
                            "memedit{}",
                            VERSION
                                .map(|vers| " v".to_string() + vers)
                                .unwrap_or(String::new())
                        ))
                        .with_name("status"),
                    )
                    .full_width(),
                ),
                |layout, size| {
                    layout.set_child_position(0, Rect::from_size((0, 0), (size.x, 1)));
                    layout.layout(size);
                },
            )
            .full_screen(),
        );
    }

    pub fn select_pid(&mut self, processes: &[ProcessRef]) {
        let mut select = SelectView::new().h_align(HAlign::Left);
        let parsed_processes = processes
            .into_iter()
            .enumerate()
            .filter(|(i, p)| !(p.command.is_empty() || p.exe.is_empty()))
            .map(|(i, p)| {
                (
                    format!("{}    {}", p.pid, &p.exe[0..32.min(p.exe.len())]),
                    i,
                )
            });

        select.add_all(parsed_processes);

        // Sets the callback for when "Enter" is pressed.
        select.set_on_submit(|cursive: &mut Cursive, index: &usize| {
            cursive.pop_layer();
            let (snd, rcv): (Sender<UiEvent>, Receiver<RenderEvent>) =
                cursive.take_user_data().unwrap();
            snd.send(UiEvent::Select(*index)).unwrap();
        });

        // Let's override the `j` and `k` keys for navigation
        let select = OnEventView::new(select)
            .on_pre_event_inner('k', |s, _| {
                let cb = s.select_up(1);
                Some(EventResult::Consumed(Some(cb)))
            })
            .on_pre_event_inner('g', |s, _| {
                let cb = s.select_up(s.len());
                Some(EventResult::Consumed(Some(cb)))
            })
            .on_pre_event_inner('G', |s, _| {
                let cb = s.select_down(s.len());
                Some(EventResult::Consumed(Some(cb)))
            })
            .on_pre_event_inner('j', |s, _| {
                let cb = s.select_down(1);
                Some(EventResult::Consumed(Some(cb)))
            })
            .on_pre_event_inner('d', |s, _| {
                let cb = s.select_down(10);
                Some(EventResult::Consumed(Some(cb)))
            })
            .on_pre_event_inner('u', |s, _| {
                let cb = s.select_up(10);
                Some(EventResult::Consumed(Some(cb)))
            });
        self.ui.add_layer(
            Dialog::around(select.scrollable().min_size((40, 10)))
                .title("Select process to inspect:"),
        );
    }

    pub fn run(&mut self) {
        self.ui.run();
    }
}
// TODO: Set pid with args
// TODO: Sorting, searching, filtering on UID, fuzzy search, marquee for long commands, etc.
// TODO: TUI menu
fn main() -> Result<()> {
    let mut siv = cursive::default();
    let (sndToUi, rcvFromMain) = std::sync::mpsc::channel::<RenderEvent>();
    let (sndToMain, rcvFromUi) = std::sync::mpsc::channel::<UiEvent>();

    // Set UI channels as user data so they are accessible in callbacks
    siv.set_user_data((sndToMain, rcvFromMain));
    let mut ui = Ui { ui: siv };
    let mut state = State {
        pid: None,
        step: Step::GetPid,
        sndToUi,
        rcvFromUi,
    };

    ui.initiate_ui();
    let processes = get_running_pids()?;

    // Need to have something responsive on UI for first callback - start with PID selection
    ui.select_pid(&processes);

    std::thread::spawn(move || state.run(&processes));
    ui.run();

    // for p in processes {
    //     println!("{}\t{}", p.pid, p.command);
    // }
    // let mut pid_string = String::new();
    // std::io::stdin().read_line(&mut pid_string)?;

    // let pid = pid_string.parse()?;
    // let process = Process::new(pid)?;
    // let maps = process.maps()?;
    // let heapmap = maps
    //     .into_iter()
    //     .find(|map| map.pathname == MMapPath::Heap)
    //     .ok_or_else(|| anyhow!("Found no Heap memory map!"))?;
    // let mut mem = process.mem().unwrap();

    // mem.seek(SeekFrom::Start(heapmap.address.0)).unwrap();
    // let mut buf = vec![0; (heapmap.address.1 - heapmap.address.0) as usize];
    // mem.read_exact(&mut buf).unwrap();
    // let idx = buf.windows(4).filter(|p| p == b"hello").unwrap();

    // restore terminal
    Ok(())
}
