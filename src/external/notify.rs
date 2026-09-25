use std::process::Command;
use notify_rust::Notification;

/// デスクトップ通知の送信
pub fn send_notification(summary: &str, body: &str) {
    let res = Notification::new()
        .appname("shard")
        .summary(summary)
        .body(body)
        .icon("download")
        .show();

    if res.is_err() {
        // notify-send CLI フォールバック
        let _ = Command::new("notify-send")
            .args(["-a", "shard", summary, body])
            .spawn();
    }
}
