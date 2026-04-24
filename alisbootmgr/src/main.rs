slint::include_modules!();
use anyhow::{Context, Result};
use efivar::VarManager;
use efivar::boot::{BootEntry as EfiBootEntry, BootEntryAttributes};
use slint::{VecModel};
use std::rc::Rc;

fn get_boot_order() -> Result<Vec<u16>> {
    let manager = efivar::system();
    match manager.read(&efivar::efi::Variable::new("BootOrder")) {
        Ok((data, _)) => {
            let mut order = Vec::new();
            for i in (0..data.len()).step_by(2) {
                if i + 1 < data.len() {
                    order.push(u16::from_le_bytes([data[i], data[i+1]]));
                }
            }
            Ok(order)
        }
        Err(_) => Ok(Vec::new()),
    }
}

fn save_boot_order(order: &[u16]) -> Result<()> {
    let mut manager = efivar::system();
    let mut data = Vec::with_capacity(order.len() * 2);
    for &id in order {
        data.extend_from_slice(&id.to_le_bytes());
    }
    manager.write(
        &efivar::efi::Variable::new("BootOrder"),
        efivar::efi::VariableFlags::NON_VOLATILE | 
        efivar::efi::VariableFlags::BOOTSERVICE_ACCESS | 
        efivar::efi::VariableFlags::RUNTIME_ACCESS,
        &data
    ).context("Failed to write BootOrder")?;
    Ok(())
}

fn main() -> Result<()> {
    let ui = AppWindow::new()?;
    
    let refresh_entries = {
        let ui_handle = ui.as_weak();
        move || {
            let ui = ui_handle.unwrap();
            let manager = efivar::system();
            let order = get_boot_order().unwrap_or_default();
            
            let mut ui_entries = Vec::new();
            for &id_num in &order {
                let id_str = format!("Boot{:04X}", id_num);
                if let Ok((data, _)) = manager.read(&efivar::efi::Variable::new(&id_str)) {
                    if let Ok(efi_entry) = EfiBootEntry::parse(data) {
                        let path_str = efi_entry.file_path_list
                            .as_ref()
                            .map(|fpl| fpl.file_path.path.clone())
                            .unwrap_or_else(|| "No path".to_string());

                        ui_entries.push(BootEntry {
                            id: id_str.into(),
                            description: efi_entry.description.into(),
                            path: path_str.into(),
                            enabled: efi_entry.attributes.contains(BootEntryAttributes::LOAD_OPTION_ACTIVE),
                        });
                    }
                }
            }
            ui.set_entries(Rc::new(VecModel::from(ui_entries)).into());
        }
    };

    refresh_entries();

    let refresh = refresh_entries.clone();
    ui.on_move_up(move |index| {
        let mut order = get_boot_order().unwrap_or_default();
        let idx = index as usize;
        if idx > 0 && idx < order.len() {
            order.swap(idx, idx - 1);
            let _ = save_boot_order(&order);
            refresh();
        }
    });

    let refresh = refresh_entries.clone();
    ui.on_move_down(move |index| {
        let mut order = get_boot_order().unwrap_or_default();
        let idx = index as usize;
        if idx < order.len() - 1 {
            order.swap(idx, idx + 1);
            let _ = save_boot_order(&order);
            refresh();
        }
    });

    let refresh = refresh_entries.clone();
    ui.on_delete_entry(move |index| {
        let mut order = get_boot_order().unwrap_or_default();
        let idx = index as usize;
        if idx < order.len() {
            let _ = order.remove(idx);
            let _ = save_boot_order(&order);
            refresh();
        }
    });

    let refresh = refresh_entries.clone();
    let ui_handle = ui.as_weak();
    ui.on_save_entry(move |description, path, is_new| {
        let ui = ui_handle.unwrap();
        let mut manager = efivar::system();
        let mut order = get_boot_order().unwrap_or_default();

        if is_new {
            let mut new_id = 0;
            while order.contains(&new_id) { new_id += 1; }
            
            // Try to clone the disk info from the first existing entry
            let mut template_fpl = None;
            for &id_num in &order {
                let id_str = format!("Boot{:04X}", id_num);
                if let Ok((data, _)) = manager.read(&efivar::efi::Variable::new(&id_str)) {
                    if let Ok(efi_entry) = EfiBootEntry::parse(data) {
                        if let Some(fpl) = efi_entry.file_path_list {
                            template_fpl = Some(fpl);
                            break;
                        }
                    }
                }
            }

            if let Some(mut fpl) = template_fpl {
                fpl.file_path.path = path.to_string();
                let efi_entry = EfiBootEntry {
                    attributes: BootEntryAttributes::LOAD_OPTION_ACTIVE,
                    description: description.to_string(),
                    file_path_list: Some(fpl),
                    optional_data: Vec::new(),
                };

                let id_str = format!("Boot{:04X}", new_id);
                let _ = manager.write(
                    &efivar::efi::Variable::new(&id_str),
                    efivar::efi::VariableFlags::NON_VOLATILE | 
                    efivar::efi::VariableFlags::BOOTSERVICE_ACCESS | 
                    efivar::efi::VariableFlags::RUNTIME_ACCESS,
                    &efi_entry.to_bytes()
                );
                order.push(new_id);
                let _ = save_boot_order(&order);
            }
        } else {
            let selected_idx = ui.get_selected_index() as usize;
            if selected_idx < order.len() {
                let id_num = order[selected_idx];
                let id_str = format!("Boot{:04X}", id_num);
                
                if let Ok((data, _)) = manager.read(&efivar::efi::Variable::new(&id_str)) {
                    if let Ok(mut efi_entry) = EfiBootEntry::parse(data) {
                        efi_entry.description = description.to_string();
                        if let Some(ref mut fpl) = efi_entry.file_path_list {
                            fpl.file_path.path = path.to_string();
                        }
                        
                        let _ = manager.write(
                            &efivar::efi::Variable::new(&id_str),
                            efivar::efi::VariableFlags::NON_VOLATILE | 
                            efivar::efi::VariableFlags::BOOTSERVICE_ACCESS | 
                            efivar::efi::VariableFlags::RUNTIME_ACCESS,
                            &efi_entry.to_bytes()
                        );
                    }
                }
            }
        }
        refresh();
    });

    ui.run()?;
    Ok(())
}
