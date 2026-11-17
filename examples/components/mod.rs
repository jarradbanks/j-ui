//! Interactive samples of the content, navigation and feedback components.

use super::*;
use j_ui::widgets::{
    Autocomplete, Badge, Banner, Card, Column, NavRow, Progress, RadioGroup, Table, Tabs, Toast,
    Tone,
};

#[derive(Default)]
pub struct Demo {
    query: String,
    selected: usize,
    tab: usize,
    toast: bool,
}

pub fn show(ui: &mut egui::Ui, demo: &mut Demo) {
    let names = [
        tr!("clip.video"),
        tr!("clip.title"),
        tr!("clip.still"),
        tr!("clip.audio"),
    ];
    ui.add(
        Autocomplete::new(&mut demo.query, &names)
            .empty(tr!("demo.no_matches"))
            .width(320.0),
    );
    ui.add(RadioGroup::new(&mut demo.selected, &names));
    ui.add(Tabs::new(&names, &mut demo.tab));
    ui.horizontal(|ui| {
        for (index, tone) in [
            Tone::Neutral,
            Tone::Info,
            Tone::Success,
            Tone::Warning,
            Tone::Danger,
        ]
        .into_iter()
        .enumerate()
        {
            ui.add(Badge::new(names[index % names.len()]).tone(tone));
        }
    });
    let banner = Banner::new(Tone::Info, tr!("update.notes")).action(tr!("demo.export"));
    if banner.show(ui).is_some_and(|action| action.clicked()) {
        demo.toast = true;
    }
    if demo.toast {
        let toast = Toast::new(tr!("update.notes"), tr!("demo.dismiss")).show(ui);
        demo.toast = !toast.dismissed;
    }
    ui.add(Progress::new(0.38, tr!("demo.export")));
    ui.horizontal_top(|ui| {
        ui.allocate_ui_with_layout(
            egui::vec2(160.0, 160.0),
            egui::Layout::top_down(egui::Align::Min),
            |ui| {
                ui.spacing_mut().item_spacing.y = 2.0;
                for (index, name) in names.iter().enumerate() {
                    if ui.add(NavRow::new(name, index == demo.selected)).clicked() {
                        demo.selected = index;
                    }
                }
            },
        );
        ui.vertical(|ui| {
            let columns = [
                Column::fill(tr!("panel.source")),
                Column::fixed(tr!("row.colour"), 100.0),
            ];
            Table::new(&columns, &mut demo.selected).show(ui, &names, |ui, row, column| {
                if column == 0 {
                    ui.label(names[row]);
                } else {
                    ui.add(Badge::new(names[row]).tone(Tone::Info));
                }
            });
        });
    });
    Card::new().show(ui, |ui| {
        egui::Frame::new()
            .inner_margin(space::INSET)
            .show(ui, |ui| {
                ui.label(theme::text(names[demo.selected], theme::title()));
                ui.label(theme::text(tr!("update.notes"), theme::body()).color(color::ink_3()));
            });
    });
}
