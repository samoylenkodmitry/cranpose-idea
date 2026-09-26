#[cranpose::composable]
fn Application() { cranpose_hot_counter::Counter(); }

fn main() {
    let endpoint = cranpose::embed::EmbedEndpoint::from_env().expect("embedded acceptance host");
    cranpose::AppLauncher::new().with_size(320, 240).run_embedded(endpoint, Application);
}
