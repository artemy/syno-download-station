use crate::entities::Task;
use crate::entities::TaskStatus::{Downloading, Seeding};
use byte_unit::{Byte, UnitType};

impl Task {
    #[must_use]
    pub fn calculate_size(&self) -> String {
        let size = Byte::from(self.size);
        format!("{:#.2}", size.get_appropriate_unit(UnitType::Decimal))
    }

    #[allow(clippy::cast_precision_loss)]
    #[must_use]
    pub fn calculate_progress(&self) -> f64 {
        self.additional
            .as_ref()
            .and_then(|additional| additional.transfer.as_ref())
            .map(|transfer| {
                let size_downloaded = transfer.size_downloaded;
                (size_downloaded as f64 / self.size as f64 * 100.0).round()
            })
            .take_if(|x| !x.is_nan())
            .unwrap_or_default()
    }

    #[must_use]
    pub fn calculate_speed(&self) -> String {
        if !matches!(self.status, Downloading) && !matches!(self.status, Seeding) {
            return String::new();
        }

        self.additional
            .as_ref()
            .and_then(|additional| additional.transfer.as_ref())
            .map(|transfer| match self.status {
                Downloading => transfer.speed_download,
                Seeding => transfer.speed_upload,
                _ => 0,
            })
            .take_if(|speed| *speed > 0u64)
            .map(|speed| {
                format!(
                    "({:#.2}/s)",
                    Byte::from(speed).get_appropriate_unit(UnitType::Decimal)
                )
            })
            .unwrap_or_default()
    }

    #[allow(clippy::cast_precision_loss, clippy::cast_possible_truncation)]
    pub fn calculate_time_left(&self) -> String {
        if !matches!(self.status, Downloading) {
            return String::new();
        }
        self.additional
            .as_ref()
            .and_then(|additional| additional.transfer.as_ref())
            .map(|transfer| match self.status {
                Downloading => {
                    let speed_download = transfer.speed_download;
                    let total_size = self.size;
                    let size_downloaded = transfer.size_downloaded;

                    if speed_download == 0 {
                        -1i64
                    } else {
                        ((total_size as f64 - size_downloaded as f64) / speed_download as f64)
                            .floor() as i64
                    }
                }
                _ => -1i64,
            })
            .map(convert_time_left)
            .map(|time_left| format!("⏳Time left: {time_left}"))
            .unwrap_or_default()
    }
}

#[must_use]
pub fn convert_time_left(input: i64) -> String {
    if input < 0 {
        return String::from("Unknown");
    }

    if input < 60 {
        return format!("{input} s");
    }

    if input < 3600 {
        let minutes = input / 60;
        let seconds = input - 60 * minutes;
        return format!("{minutes} m {seconds} s");
    }

    if input < 86400 {
        let hours = input / 3600;
        let minutes = (input - hours * 3600) / 60;
        return format!("{hours} h {minutes} m");
    }

    let days = input / 86400;
    let hours = (input - days * 86400) / 3600;
    let minutes = (input - days * 86400 - hours * 3600) / 60;
    format!("{days} d {hours} h {minutes} m")
}

#[cfg(test)]
mod tests {
    // Note this useful idiom: importing names from outer (for mod tests) scope.
    use super::*;
    use crate::entities::{AdditionalTaskInfo, Transfer};

    impl Task {
        fn create_test_task() -> Task {
            Task {
                id: String::from("123"),
                username: String::from("Bob"),
                task_type: String::from("bt"),
                title: String::from("Ubuntu 16.04"),
                size: 1_234_567_890,
                status: Downloading,
                status_extra: None,
                additional: Some(AdditionalTaskInfo {
                    transfer: Some(Transfer {
                        speed_download: 98765,
                        ..Default::default()
                    }),
                    ..Default::default()
                }),
            }
        }
    }

    #[test]
    fn test_calculate_size() {
        let task = Task::create_test_task();
        assert_eq!("1.23 GB", task.calculate_size());
    }

    #[test]
    fn test_calculate_speed() {
        // Test with downloading
        let mut task = Task::create_test_task();
        assert_eq!("(98.77 KB/s)", task.calculate_speed());
        // Test with uploading
        task.status = Seeding;
        task.additional.as_mut().unwrap().transfer.as_mut().unwrap().speed_download = 0;
        task.additional.as_mut().unwrap().transfer.as_mut().unwrap().speed_upload = 45678;
        assert_eq!("(45.68 KB/s)", task.calculate_speed());
    }

    #[test]
    fn test_calculate_time_left() {
        let task = Task::create_test_task();
        assert_eq!("⏳Time left: 3 h 28 m", task.calculate_time_left());
    }

}
