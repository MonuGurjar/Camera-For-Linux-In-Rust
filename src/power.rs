use zbus::{Connection, proxy};
use futures_lite::stream::StreamExt;

#[proxy(
    interface = "org.freedesktop.login1.Manager",
    default_service = "org.freedesktop.login1",
    default_path = "/org/freedesktop/login1"
)]
trait Login1Manager {
    #[zbus(signal)]
    fn prepare_for_sleep(&self, start: bool) -> zbus::Result<()>;
}

pub async fn monitor_power_events() -> zbus::Result<()> {
    let connection = Connection::system().await?;
    let proxy = Login1ManagerProxy::new(&connection).await?;
    
    let mut stream = proxy.receive_prepare_for_sleep().await?;
    
    while let Some(signal) = stream.next().await {
        let args = signal.args()?;
        if args.start {
            println!("System is preparing for sleep. Suspending camera...");
            // Send suspend command to camera backend
        } else {
            println!("System resumed from sleep. Waking camera...");
            // Send resume command to camera backend
        }
    }
    
    Ok(())
}
