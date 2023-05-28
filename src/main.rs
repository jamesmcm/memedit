mod ui;

use std::io::{Read, Seek, SeekFrom};
use std::{sync::mpsc::Receiver, sync::mpsc::Sender};

use anyhow::{anyhow, Result};

use memedit::ProcessRef;
use procfs::process::{MMapPath, MemoryMaps};
use procfs::process::{MemoryMap, Process};

const VERSION: Option<&str> = option_env!("CARGO_PKG_VERSION");
pub struct State {
    pid: Option<i32>,
    step: Step,
    snd_to_ui: Sender<RenderEvent>,
    rcv_from_ui: Receiver<UiEvent>,
    process: Option<Process>,
    memory_map: Option<MemoryMap>,
    // mapname, range, memdump, addresses of interest
}
impl State {
    // TODO: Separate these steps in actual state machine
    pub fn run(&mut self, initial_processes: &[ProcessRef]) -> Result<()> {
        self.pid = match self.rcv_from_ui.recv()? {
            UiEvent::Select(index) => Some(initial_processes[index].pid),
            _ => {
                anyhow::bail!("Failed to set PID");
            }
        };
        eprintln!("PID: {}", self.pid.unwrap());

        // TODO: Make heap default somehow
        self.step = Step::GetMmap;
        let process = Process::new(self.pid.unwrap())?;
        let maps = process.maps()?;
        self.process = Some(process);
        self.snd_to_ui
            .send(RenderEvent::GetMMap(maps.memory_maps.clone()))?; // TODO: Avoid clone

        self.memory_map = match self.rcv_from_ui.recv()? {
            UiEvent::Select(index) => Some(maps.memory_maps[index].clone()), // TODO: Avoid clone
            _ => {
                anyhow::bail!("Failed to set memory map");
            }
        };
        eprintln!("memory map: {:?}", self.memory_map.as_ref().unwrap());

        self.step = Step::LoadMem;
        // TODO: Zero copy version - re-seeking, or can we read slice directly with process_vm_readv ?
        let mut mem = self.process.as_ref().unwrap().mem().unwrap();
        mem.seek(SeekFrom::Start(self.memory_map.as_ref().unwrap().address.0))
            .unwrap();
        let mut buf = vec![
            0;
            (self.memory_map.as_ref().unwrap().address.1
                - self.memory_map.as_ref().unwrap().address.0) as usize
        ];
        mem.read_exact(&mut buf).unwrap();
        // eprintln!(
        //     "mem: {:?}",
        //     buf.iter().map(|b| *b as char).collect::<Vec<char>>()
        // );

        self.snd_to_ui.send(RenderEvent::MainScreen(
            self.memory_map.as_ref().unwrap().address.0,
            self.memory_map.as_ref().unwrap().address.1,
            buf.clone(),
        ))?; // TODO: Pass reference

        loop {
            let event = self.rcv_from_ui.recv()?;

            match event {
                UiEvent::Quit => {
                    // std::process::exit(0);
                    break;
                } // Enough to break here?
                UiEvent::Search(search_string) => {
                    // Handle search
                    eprintln!("search string: {}", &search_string);

                    self.snd_to_ui.send(RenderEvent::Dummy)?;
                }
                _ => eprintln!("Received unexpected UI event: {:?}", &event),
            }
        }

        Ok(())
    }
}

pub enum Step {
    GetPid,
    GetMmap,
    LoadMem,
    WaitLoop,
    SearchMem, // Add search parameters - domain, predicate (changed, unchanged, equal, <, >), data type (int, float, string)
    WriteMem,  // Add write parameters - address, length, infer data type, endianness?, mass write?
}

#[derive(Debug)]
pub enum UiEvent {
    Select(usize),
    Search(String), // TODO: Handle other types - string, float, pointers
    Quit,
}

// TODO: Avoid cloning via Arc Mutex etc.
pub enum RenderEvent {
    GetPid(Vec<ProcessRef>),
    GetMMap(Vec<MemoryMap>),
    MainScreen(u64, u64, Vec<u8>),
    Dummy,
}

// TODO: Set pid with args
// TODO: Sorting, searching, filtering on UID, fuzzy search, marquee for long commands, etc.
// TODO: TUI menu
fn main() -> Result<()> {
    let mut siv = cursive::default();
    let (sndToUi, rcvFromMain) = std::sync::mpsc::channel::<RenderEvent>();
    let (sndToMain, rcvFromUi) = std::sync::mpsc::channel::<UiEvent>();

    // Set UI channels as user data so they are accessible in callbacks
    // TODO: Make this a struct for adding more UI state
    siv.set_user_data((sndToMain, rcvFromMain));
    let mut ui = ui::Ui { ui: siv };
    let mut state = State {
        pid: None,
        step: Step::GetPid,
        snd_to_ui: sndToUi,
        rcv_from_ui: rcvFromUi,
        process: None,
        memory_map: None,
    };

    ui.initiate_ui(VERSION);
    let processes = memedit::get_running_pids()?;

    // Need to have something responsive on UI for first callback - start with PID selection
    ui::select_pid(&mut ui.ui, &processes);

    std::thread::spawn(move || state.run(&processes));
    ui.run();

    // for p in processes {
    //     println!("{}\t{}", p.pid, p.command);
    // }
    // let mut pid_string = String::new();
    // std::io::stdin().read_line(&mut pid_string)?;

    // let pid = pid_string.parse()?;
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
