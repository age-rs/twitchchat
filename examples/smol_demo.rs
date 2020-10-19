use twitchchat::{
    commands,
    runner::{
        idle_detection_loop, //
        respond_to_idle_events,
        wait_until_ready,
        Activity,
        ActivitySender,
    },
    writer::MpscWriter,
    BoxedAsyncDecoder, BoxedAsyncEncoder, UserConfig,
};

// this is a helper module to reduce code deduplication
mod include;
use crate::include::{channels_to_join, get_user_config, main_loop};

async fn connect(
    user_config: &UserConfig,
    channels: &[String],
) -> anyhow::Result<(BoxedAsyncDecoder, BoxedAsyncEncoder)> {
    // create a connector using ``smol``, this connects to Twitch.
    // you can provide a different address with `connect_custom`
    let mut stream = twitchchat_smol::connect_twitch().await?;
    println!("we're connecting!");

    // this method will block until you're ready
    // it'll return any messages you missed while it was waiting
    let (identity, _missed_messages) = wait_until_ready(&mut stream, user_config).await?;
    println!("..and we're connected");

    // and the identity Twitch gave you
    println!("our identity: {:#?}", identity);

    // make an decoder and encoder
    let (decode, mut encode) = twitchchat::make_boxed_pair(stream);

    for channel in channels {
        // the runner itself has 'blocking' join/part to ensure you join/leave a channel.
        // these two methods return whether the connection was closed early.
        // we'll ignore it for this demo
        println!("attempting to join '{}'", channel);
        // NOTE: this doesn't actually block for your join
        // you can use `runner::wait_for` to build state tracking
        let _ = encode.join(channel).await?;
    }

    Ok((decode, encode))
}

async fn do_some_stuff(writer: MpscWriter, channels: Vec<String>) {
    println!("in 10 seconds we'll exit");
    smol::Timer::after(std::time::Duration::from_secs(10)).await;

    // send one final message to all channels
    for channel in channels {
        let cmd = commands::privmsg(&channel, "goodbye, world");
        writer.send(cmd).unwrap();
    }

    println!("sending quit signal");
    writer.shutdown().await;
}

fn setup_idle_detection(writer: MpscWriter) -> ActivitySender {
    let (activity, input) = Activity::pair();
    let (tx, rx) = flume::unbounded();
    // spawn off the idle detection loop
    smol::spawn(idle_detection_loop(input, tx)).detach();
    // and set up the responder loop
    smol::spawn(respond_to_idle_events(writer, rx)).detach();
    // and return the handle for interaction with the loop
    activity
}

async fn start(user_config: &UserConfig, channels: Vec<String>) -> anyhow::Result<()> {
    // connect and join the provided channels
    let (decoder, encoder) = connect(&user_config, &channels).await?;

    // you can make a writer from the encoder -- this is clonable and thread safe
    let writer = MpscWriter::from_async_encoder(encoder);

    // spawn something off in the background that'll exit in 10 seconds
    smol::spawn(do_some_stuff(writer.clone(), channels.clone())).detach();

    // you can encode all sorts of 'commands'
    for channel in &channels {
        writer.send(commands::privmsg(channel, "hello world!"))?;
    }

    // you can set up idle detection, as well
    let activity = setup_idle_detection(writer.clone());

    // and then start 'main' loop
    println!("starting main loop");
    main_loop(decoder, writer, activity).await;

    Ok(())
}

fn main() -> anyhow::Result<()> {
    // create a user configuration
    let user_config = get_user_config()?;
    // get some channels to join from the environment
    let channels = channels_to_join()?;

    // and start it
    let fut = start(&user_config, channels);
    smol::block_on(fut)
}
