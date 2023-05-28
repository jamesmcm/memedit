use cursive::{
    align::HAlign,
    event::EventResult,
    traits::{Nameable as _, Resizable as _},
    view::{Scrollable, View as _},
    views::{
        Dialog, EditView, FixedLayout, Layer, LinearLayout, OnEventView, OnLayoutView, SelectView,
        TextArea, TextView,
    },
    Cursive, CursiveRunnable, {Rect, Vec2},
};

use crate::{RenderEvent, UiEvent}; // TODO: Move binary to other crate

use procfs::process::MemoryMap;

use memedit::ProcessRef;

use std::{sync::mpsc::Receiver, sync::mpsc::Sender};

pub struct Ui {
    pub ui: CursiveRunnable,
}

impl Ui {
    pub fn initiate_ui(&mut self, version: Option<&str>) {
        self.ui.add_global_callback('q', Cursive::quit);

        self.ui.screen_mut().add_transparent_layer(
            OnLayoutView::new(
                FixedLayout::new().child(
                    Rect::from_point(Vec2::zero()),
                    Layer::new(
                        TextView::new(format!(
                            "memedit{}",
                            version
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

    // Select List for PID and MMap

    pub fn run(&mut self) {
        self.ui.run();
    }
}

pub fn select_pid(ui: &mut Cursive, processes: &[ProcessRef]) {
    let mut select = SelectView::new().h_align(HAlign::Left);
    let parsed_processes = processes
        .iter()
        .enumerate()
        .filter(|(_i, p)| !(p.command.is_empty() || p.exe.is_empty()))
        .map(|(i, p)| {
            (
                format!("{}    {}", p.pid, &p.exe[0..32.min(p.exe.len())]),
                i,
            )
        });

    select.add_all(parsed_processes);
    let select = common_list_settings(select);
    ui.add_layer(
        Dialog::around(select.scrollable().min_size((40, 10))).title("Select process to inspect:"),
    );
}

pub fn select_mmap(ui: &mut Cursive, maps: &[MemoryMap]) {
    let mut select = SelectView::new().h_align(HAlign::Left);
    let parsed_maps = maps.iter().enumerate().map(|(i, p)| {
        (
            format!(
                "({:#x}-{:#x})    {:?}",
                p.address.0, p.address.1, p.pathname
            ),
            i,
        )
    });

    select.add_all(parsed_maps);

    let select = common_list_settings(select);
    ui.add_layer(
        Dialog::around(select.scrollable().min_size((40, 10)))
            .title("Select memory map to inspect:"),
    );
}

pub fn load_main_screen(ui: &mut Cursive, mem: (u64, u64, &[u8])) {
    // TODO: Add address lines, make the display buffer, make text read-only, make cursor move only to next byte (two chars and ignore whitespace)
    // TODO: Add status bar to bottom showing currently selected address
    let text_area = TextArea::new()
        .content(
            (mem.2)[0..4000]
                .iter()
                .map(|x| format!("{:02x}", x))
                .collect::<Vec<String>>()
                .join(" "),
        ) // TODO: Fix so many allocations
        .with_name("hex_values")
        // .fixed_width(30)
        .min_height(5)
        .scrollable();

    // let view = ResizedView::with_full_screen(text_area);
    let view = OnEventView::new(text_area)
        .on_pre_event('q', |c| c.quit())
        .on_pre_event('/', display_search_screen);
    ui.add_layer(view);
}

pub fn display_search_screen(ui: &mut Cursive) {
    // TODO: Get type
    let dialog = Dialog::around(
        LinearLayout::vertical()
            .child(TextView::new("Enter value to search:"))
            .child(
                EditView::new()
                    .with_name("search_field")
                    .fixed_height(1)
                    .fixed_width(16),
            ),
    )
    .button("Cancel", |c| {
        c.pop_layer();
    })
    .button("Ok", |c| {
        let search = c
            .find_name::<EditView>("search_field")
            .unwrap()
            .get_content();
        let (snd, rcv): (Sender<UiEvent>, Receiver<RenderEvent>) = c.take_user_data().unwrap();
        snd.send(UiEvent::Search(search.to_string())).unwrap();
        let to_render = rcv.recv().unwrap();
        c.pop_layer(); // TODO: Move this to state-based destructor?
        c.set_user_data((snd, rcv)); // TODO: Make this guard-based ?
        handle_render_event(c, to_render);
    });

    ui.add_layer(dialog);
}

pub fn common_list_settings(mut select: SelectView<usize>) -> OnEventView<SelectView<usize>> {
    // Sets the callback for when "Enter" is pressed.
    select.set_on_submit(|cursive: &mut Cursive, index: &usize| {
        let (snd, rcv): (Sender<UiEvent>, Receiver<RenderEvent>) =
            cursive.take_user_data().unwrap();
        snd.send(UiEvent::Select(*index)).unwrap();
        let to_render = rcv.recv().unwrap();
        cursive.pop_layer(); // TODO: Move this to state-based destructor?
        cursive.set_user_data((snd, rcv)); // TODO: Make this guard-based ?
        handle_render_event(cursive, to_render);
    });

    // Let's override the `j` and `k` keys for navigation

    OnEventView::new(select)
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
        })
}

pub fn handle_render_event(cursive: &mut Cursive, render_event: RenderEvent) {
    match render_event {
        RenderEvent::GetPid(processes) => select_pid(cursive, &processes),
        RenderEvent::GetMMap(maps) => select_mmap(cursive, &maps),
        RenderEvent::MainScreen(start, end, mem) => load_main_screen(cursive, (start, end, &mem)),
        RenderEvent::Dummy => {}
    }
}
