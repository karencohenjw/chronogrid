mod ical;
mod legacy;
mod recurrence;
mod temporal;

fn main() -> anyhow::Result<()> {
    legacy::run()
}
