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
    // mapname, range, memdump, addresses of interest
}

pub enum Step {
    GetPid,
    GetMmap,
    LoadMem,
    StartFilterMem,
    EndFilterMem,
    StartWriteMem,
    EndWriteMem,
}

pub struct App {
    ui: CursiveRunnable,
    state: State,
    step: Step,
}

impl App {
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

    pub fn render_ui(&mut self) -> Result<()> {
        match self.step {
            Step::GetPid => {
                let processes = get_running_pids()?;
                let mut select = SelectView::new().h_align(HAlign::Left);
                let strings = processes
                    .into_iter()
                    .filter(|p| !p.command.is_empty())
                    .map(|p| format!("{}    {}", p.pid, &p.command[0..32.min(p.command.len())]))
                    .collect::<Vec<String>>();

                select.add_all_str(&strings);

                // Sets the callback for when "Enter" is pressed.
                select.set_on_submit(|cursive, s: &str| {
                    cursive.pop_layer();
                    println!("{}", s);
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

                Ok(())
            }
            _ => Ok(()),
        }
    }
}
// TODO: Set pid with args
// TODO: Sorting, searching, filtering on UID, fuzzy search, marquee for long commands, etc.
// TODO: TUI menu
fn main() -> Result<()> {
    let mut siv = cursive::default();
    let mut state = State { pid: None };
    let mut app = App {
        ui: siv,
        state,
        step: Step::GetPid,
    };

    // TODO: Let UI callbacks communicate with state machine via channels
    app.initiate_ui();
    app.render_ui()?;
    app.ui.run();

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
