#![forbid(unsafe_code)]

use iui::prelude::*;
use iui::controls::*;

pub fn run() {
    let ui = UI::init()
        .expect("Failed to initialize UI");

    let mut win = Window::new(&ui, "Keep Keeping – Started", 350, 250, WindowType::NoMenubar);

    let mut box_v = VerticalBox::new(&ui);
    box_v.set_padded(&ui, true);

    let mut box_path1 = HorizontalBox::new(&ui);
    let mut box_path2 = HorizontalBox::new(&ui);

    box_path2.set_padded(&ui, true);
    box_path1.set_padded(&ui, true);

    let label_dir1 = Label::new(&ui, "Path 1");
    let entry_dir1 = Entry::new(&ui);
    let mut button_select_dir1 = Button::new(&ui, "…");
    button_select_dir1.on_clicked(&ui, {
        let mut entry_dir1 = entry_dir1.clone();
        let ui = ui.clone();
        move |_| {
            if let Some(path) = pick_folder() {
                entry_dir1.set_value(&ui, &path);
            }
        }
    });

    let label_dir2 = Label::new(&ui, "Path 2");
    let entry_dir2 = Entry::new(&ui);
    let mut button_select_dir2 = Button::new(&ui, "…");
    button_select_dir2.on_clicked(&ui, {
        let mut entry_dir2 = entry_dir2.clone();
        let ui = ui.clone();
        move |_| {
            if let Some(path) = pick_folder() {
                entry_dir2.set_value(&ui, &path);
            }
        }
    });

    box_path1.append(&ui, label_dir1, LayoutStrategy::Compact);
    box_path2.append(&ui, label_dir2, LayoutStrategy::Compact);

    box_path1.append(&ui, entry_dir1.clone(), LayoutStrategy::Stretchy);
    box_path2.append(&ui, entry_dir2.clone(), LayoutStrategy::Stretchy);

    box_path1.append(&ui, button_select_dir1, LayoutStrategy::Compact);
    box_path2.append(&ui, button_select_dir2, LayoutStrategy::Compact);

    let mut button_synchronize = Button::new(&ui, "Synchronize");
    button_synchronize.on_clicked(&ui, {
        let mut win = win.clone();
        let ui = ui.clone();
        move |_| {
            use std::path::Path;

            let path1 = &entry_dir1.value(&ui);
            let path2 = &entry_dir2.value(&ui);

            let path1 = Path::new(path1);
            let path2 = Path::new(path2);

            if !path1.exists() {
                if path2.exists() {
                    win.set_title(&ui, "Keep Keeping – Path 1 does not exist");
                } else {
                    win.set_title(&ui, "Keep Keeping – Path 1 & Path 2 do not exist");
                }
            } else if !path2.exists() {
                win.set_title(&ui, "Keep Keeping – Path 2 does not exist");
            } else {
                use super::lib::synchronize;

                win.set_title(&ui, "Keep Keeping – Synchronizing…");
                let _ = synchronize(path1, path2);
                win.set_title(&ui, "Keep Keeping – Done");
            }
        }
    });
    
    box_v.append(&ui, box_path1, LayoutStrategy::Compact);
    box_v.append(&ui, box_path2, LayoutStrategy::Compact);
    box_v.append(&ui, button_synchronize, LayoutStrategy::Compact);


    win.set_child(&ui, box_v);
    win.on_closing(&ui, {
        let ui = ui.clone();
        move |_| ui.quit()
    });
    win.show(&ui);
    ui.main();
}

pub fn pick_folder() -> Option<String> {
    if let Ok(response) = nfd::open_pick_folder(None) {
        if let nfd::Response::Okay(path) = response {
            Some(path)
        } else {
            None
        }
    } else {
        None
    }
}
