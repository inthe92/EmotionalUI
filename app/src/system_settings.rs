//! System IO stays in the service worker; callbacks only enqueue intent.
use crate::{state::State, Home};
use emotional_core::controls::Action;
use emotional_services::system::{self, BatteryState, BluetoothState, WifiState};
use slint::{ComponentHandle, Model, ModelRc, SharedString, VecModel};

fn percent(value: Option<u8>) -> String {
    value
        .map(|v| format!("{v} %"))
        .unwrap_or_else(|| "Desconocido".into())
}

fn panel_focus(index: usize, action: Action) -> usize {
    match (index, action) {
        (0, Action::Right) => 1,
        (1, Action::Left) => 0,
        (0 | 1, Action::Down) => 2,
        (0 | 1, Action::Up) => index,
        (2, Action::Up) => 0,
        (_, Action::Up) => index.saturating_sub(1),
        (_, Action::Down) => (index + 1).min(6),
        _ => index,
    }
}

impl State {
    pub(super) fn refresh_system(&mut self, ui: &Home) {
        let drop_changed = self.poll_drop();
        if let Some(next) = self
            .system_service
            .as_ref()
            .and_then(|service| service.latest())
        {
            if self.system_snapshot.as_ref() != Some(&next) {
                self.system_note = None;
                self.system_snapshot = Some(next);
                self.paint_system(ui);
            }
        }
        if drop_changed { self.paint_system(ui); }
    }

    pub(super) fn paint_system(&self, ui: &Home) {
        let mut values = vec![SharedString::from("Desconocido"); 5];
        let mut known = vec![false; 5];
        let mut active = vec![false; 5];
        let mut help = "Consultando el sistema…".to_string();
        let mut enabled = false;
        let mut levels = vec![-1; 2];
        let mut battery_percent = -1;
        let mut battery_charging = false;
        if let Some(s) = &self.system_snapshot {
            battery_percent = s.battery.percent.map(i32::from).unwrap_or(-1);
            battery_charging = s.battery.state == BatteryState::Charging;
            levels = vec![
                s.volume.map(i32::from).unwrap_or(-1),
                s.brightness.applied_percent.map(i32::from).unwrap_or(-1),
            ];
            values[0] = match s.wifi.state {
                WifiState::Connected | WifiState::Disconnected => "On",
                WifiState::Disabled => "Off",
                WifiState::Unavailable => "No disponible",
                WifiState::Error => "Error",
                WifiState::Unknown => "—",
            }
            .into();
            values[1] = match s.bluetooth.state {
                BluetoothState::Enabled => "On",
                BluetoothState::Disabled | BluetoothState::ServiceInactive => "Off",
                BluetoothState::Unavailable => "No disponible",
                BluetoothState::Error => "Error",
                BluetoothState::Unknown => "—",
            }
            .into();
            values[2] = if s.volume_muted == Some(true) {
                "Silenciado".into()
            } else {
                percent(s.volume).into()
            };
            values[3] = match (s.brightness.applied_percent, s.brightness.requested_percent) {
                (Some(a), Some(r)) if a != r => format!("{a} % · sol. {r} %"),
                (a, _) => percent(a),
            }
            .into();
            values[4] = format!(
                "{}{}{}",
                percent(s.battery.percent),
                if s.battery.percent.is_some_and(|p| p <= 15) {
                    " · Baja"
                } else {
                    ""
                },
                match s.battery.state {
                    BatteryState::Charging => " · Cargando",
                    BatteryState::Discharging => " · Descargando",
                    BatteryState::Full => " · Completa",
                    BatteryState::NotCharging => " · Sin cargar",
                    BatteryState::Unknown => "",
                }
            )
            .into();
            // Capsule order is Wi-Fi, battery, Bluetooth, volume, brightness.
            known = vec![
                !matches!(
                    s.wifi.state,
                    WifiState::Unknown | WifiState::Unavailable | WifiState::Error
                ),
                s.battery.percent.is_some(),
                !matches!(
                    s.bluetooth.state,
                    BluetoothState::Unknown | BluetoothState::Unavailable | BluetoothState::Error
                ),
                s.volume.is_some(),
                s.brightness.applied_percent.is_some(),
            ];
            active = vec![
                s.wifi.state == WifiState::Connected,
                s.battery.percent.is_some(),
                s.bluetooth.state == BluetoothState::Enabled,
                s.volume.is_some_and(|v| v > 0) && s.volume_muted != Some(true),
                s.brightness.applied_percent.is_some_and(|v| v > 0),
            ];
            enabled = s.allow_changes && !self.trial_mode;
            help = s.message.clone().unwrap_or_else(|| {
                if enabled {
                    "◀ / ▶ ajustar · A elegir · B volver".into()
                } else {
                    "Sólo lectura. Los controles requieren el modo funcional.".into()
                }
            });
        }
        if let Some(note) = &self.system_note {
            help = note.clone();
        }
        if !ui.get_system_values().iter().eq(values.iter().cloned()) {
            ui.set_system_values(ModelRc::new(VecModel::from(values)));
        }
        if !ui.get_system_known().iter().eq(known.iter().copied()) {
            ui.set_system_known(ModelRc::new(VecModel::from(known)));
        }
        if !ui.get_system_active().iter().eq(active.iter().copied()) {
            ui.set_system_active(ModelRc::new(VecModel::from(active)));
        }
        if !ui.get_system_levels().iter().eq(levels.iter().copied()) {
            ui.set_system_levels(ModelRc::new(VecModel::from(levels)));
        }
        ui.set_system_battery_percent(battery_percent);
        ui.set_system_battery_charging(battery_charging);
        ui.set_system_selected(self.system_index as i32);
        ui.set_system_help(help.into());
        ui.set_system_controls_enabled(enabled);
        ui.set_drop_open(self.drop_open);
        ui.set_drop_image(self.drop_image.clone());
        ui.set_drop_message(self.drop_message.clone().into());
    }

    pub(super) fn system_action(&mut self, ui: &Home, action: Action) {
        if self.drop_open {
            if action == Action::Confirm { self.stop_drop(); }
            self.paint(ui);
            return;
        }
        match action {
            Action::Up | Action::Down => self.system_index = panel_focus(self.system_index, action),
            Action::Left | Action::Right if self.system_index < 2 => {
                self.system_index = panel_focus(self.system_index, action);
            }
            Action::Confirm if self.system_index == 6 => {
                self.close_modal();
                self.navigate(ui, 3);
            }
            Action::Confirm if self.system_index == 5 => self.start_drop(ui),
            Action::Left | Action::Right if matches!(self.system_index, 2 | 3) => {
                self.adjust_system(action == Action::Right);
            }
            Action::Confirm if matches!(self.system_index, 2 | 3) => {
                self.system_note = Some("Usa izquierda y derecha para ajustar.".into());
            }
            Action::Confirm if matches!(self.system_index, 0 | 1) => {
                self.open_connectivity_settings(ui, self.system_index == 1);
            }
            Action::Confirm => {
                self.system_note =
                    Some("Batería leída del controlador; no se estima desde el voltaje.".into());
            }
            _ => {}
        }
        self.paint(ui);
    }

    fn start_drop(&mut self, ui: &Home) {
        self.drop_open = true;
        self.drop_message = "Preparando la conexión local…".into();
        self.drop_image = slint::Image::default();
        self.drop_ready = false;
        if self.trial_mode || self.drop_session.is_some() {
            self.drop_message = "La transferencia no está disponible en este momento.".into();
            return;
        }
        let Some(directory) = self.launch.as_ref().map(|c| c.directory.clone()) else {
            self.drop_message = "La transferencia requiere la versión funcional.".into();
            return;
        };
        let Some(profile) = self.drop_profile.clone() else {
            self.drop_message = "No hay una biblioteca compatible configurada.".into();
            return;
        };
        let weak = ui.as_weak();
        self.drop_session = Some(crate::rom_drop::Session::start(directory, profile, move || {
            let _ = weak.upgrade_in_event_loop(|ui| ui.invoke_system_refresh());
        }));
    }

    pub(super) fn stop_drop(&mut self) {
        self.drop_open = false;
        self.drop_image = slint::Image::default();
        if let Some(session) = &self.drop_session { session.stop(); }
    }

    fn poll_drop(&mut self) -> bool {
        let mut events = Vec::new();
        if let Some(session) = &self.drop_session {
            while let Ok(event) = session.events.try_recv() { events.push(event); }
        }
        let changed = !events.is_empty();
        for event in events {
            match event {
                crate::rom_drop::Event::Ready(url) => match crate::rom_drop::qr_image(&url) {
                    Ok(image) if self.drop_open => {
                        self.drop_image = image;
                        self.drop_ready = true;
                        self.drop_message = "Escanea con el móvil conectado a la misma red.".into();
                    }
                    _ => if let Some(session) = &self.drop_session { session.stop(); },
                },
                crate::rom_drop::Event::Failed => {
                    self.drop_message = "No se pudo iniciar. Comprueba la red local.".into();
                }
                crate::rom_drop::Event::Closed => {
                    if self.drop_ready { self.drop_refresh = true; }
                    self.drop_session = None;
                    if self.drop_open && self.drop_ready {
                        self.drop_message = "La conexión se ha cerrado.".into();
                        self.drop_image = slint::Image::default();
                    }
                    self.drop_ready = false;
                }
            }
        }
        changed
    }

    fn adjust_system(&mut self, forward: bool) {
        let Some(s) = &self.system_snapshot else {
            return;
        };
        if self.trial_mode || !s.allow_changes {
            self.system_note = Some("Modo de sólo lectura: el sistema no se ha cambiado.".into());
            return;
        }
        let current = if self.system_index == 2 {
            s.volume
        } else {
            s.brightness.requested_percent
        };
        let Some(current) = current else {
            self.system_note = Some("El controlador no ofrece un valor fiable.".into());
            return;
        };
        let next = if forward {
            current.saturating_add(5).min(100)
        } else {
            current.saturating_sub(5)
        };
        if next == current {
            return;
        }
        self.set_system_level(self.system_index, next);
    }

    pub(super) fn set_system_level(&mut self, index: usize, next: u8) {
        if !matches!(index, 2 | 3) || next > 100 {
            return;
        }
        let Some(s) = &self.system_snapshot else {
            return;
        };
        if self.trial_mode || !s.allow_changes {
            self.system_note = Some("Modo de sólo lectura: el sistema no se ha cambiado.".into());
            return;
        }
        let known = if index == 2 {
            s.volume.is_some()
        } else {
            s.brightness.requested_percent.is_some()
        };
        if !known {
            return;
        }
        self.system_index = index;
        if (index == 2 && s.volume == Some(next))
            || (index == 3
                && s.brightness.requested_percent == Some(next)
                && s.brightness.applied_percent == Some(next))
        {
            self.system_note = None;
            return;
        }
        let request = if index == 2 {
            system::Action::Volume(next)
        } else {
            system::Action::Brightness {
                percent: next,
                safe_test: s.remote_session,
            }
        };
        self.system_note = Some(
            if self
                .system_service
                .as_ref()
                .is_some_and(|service| service.request(request).is_ok())
            {
                "Aplicando y comprobando el resultado…"
            } else {
                "No se ha podido solicitar el cambio. El valor mostrado sigue siendo el leído."
            }
            .into(),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn connectivity_tiles_follow_horizontal_geometry() {
        assert_eq!(panel_focus(0, Action::Right), 1);
        assert_eq!(panel_focus(1, Action::Left), 0);
        assert_eq!(panel_focus(0, Action::Left), 0);
        assert_eq!(panel_focus(1, Action::Right), 1);
        for index in 0..2 {
            assert_eq!(panel_focus(index, Action::Down), 2);
        }
        assert_eq!(panel_focus(2, Action::Up), 0);
    }
    #[test]
    fn slider_rows_keep_vertical_order_and_limits() {
        for index in 2..6 {
            assert_eq!(panel_focus(index, Action::Down), index + 1);
        }
        for index in 3..=6 {
            assert_eq!(panel_focus(index, Action::Up), index - 1);
        }
        assert_eq!(panel_focus(6, Action::Down), 6);
        assert_eq!(panel_focus(2, Action::Right), 2);
    }
}
