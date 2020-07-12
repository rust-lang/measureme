use measureme::{
    event_id::SEPARATOR_BYTE, file_header, EventId, ProfilerFiles, RawEvent, SerializationSink,
    StringComponent, StringId, StringTableBuilder,
};
use std::{error::Error, path::Path, sync::Arc, time::Instant};
use tracing_core::{
    field::{Field, Visit},
    span::{Attributes, Id, Record},
    Event, Metadata,
};
use tracing_subscriber::{
    layer::{Context, Layer},
    registry,
};

pub struct MeasuremeLayer<Sink: SerializationSink> {
    event_sink: Sink,
    string_table: StringTableBuilder<Sink>,
    start_time: Instant,
}

impl<Sink: SerializationSink> MeasuremeLayer<Sink> {
    pub fn new<P: AsRef<Path>>(
        path: P,
    ) -> Result<MeasuremeLayer<Sink>, Box<dyn Error + Send + Sync>> {
        let paths = ProfilerFiles::new(path);
        let event_sink = Sink::from_path(paths.events_file.as_ref())?;
        file_header::write_file_header(&event_sink, file_header::FILE_MAGIC_EVENT_STREAM);
        let string_table = StringTableBuilder::new(
            Arc::new(Sink::from_path(paths.string_data_file.as_ref())?),
            Arc::new(Sink::from_path(paths.string_index_file.as_ref())?),
        );
        let mut args = String::new();
        for arg in std::env::args() {
            args.push_str(&arg.escape_default().to_string());
            args.push(' ');
        }
        string_table.alloc_metadata(&*format!(
            r#"{{ "start_time": {}, "process_id": {}, "cmd": "{}" }}"#,
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos(),
            std::process::id(),
            args,
        ));

        Ok(Self {
            event_sink,
            string_table,
            start_time: Instant::now(),
        })
    }

    fn nanos_since_start(&self) -> u64 {
        self.start_time.elapsed().as_nanos() as _
    }

    fn sid_from_metadata(&self, metadata: &'static Metadata) -> StringId {
        let line = metadata.line().map(|l| l.to_string());
        let mut init_kv = vec![StringComponent::Value(metadata.name())];
        if let Some(file) = metadata.file() {
            init_kv.push(StringComponent::Value(SEPARATOR_BYTE));
            init_kv.push(StringComponent::Value("filename=")); // TODO: use Ref
            init_kv.push(StringComponent::Value(file));
        }
        if let Some(ref line) = line {
            init_kv.push(StringComponent::Value(SEPARATOR_BYTE));
            init_kv.push(StringComponent::Value("line=")); // TODO: use Ref
            init_kv.push(StringComponent::Value(line));
        }
        self.string_table.alloc(&init_kv[..])
    }
}

struct EventKind(StringId);
struct SpanEventId(StringId);
struct SpanTimestamp(u64);
struct FieldVisitor<'a, Sink: SerializationSink>(&'a StringTableBuilder<Sink>, StringId);

impl<Sink, Subscriber> Layer<Subscriber> for MeasuremeLayer<Sink>
where
    Sink: SerializationSink,
    Subscriber: tracing_core::Subscriber + for<'a> registry::LookupSpan<'a>,
{
    fn new_span(&self, attrs: &Attributes, id: &Id, ctx: Context<Subscriber>) {
        if let Some(span) = ctx.span(id) {
            let metadata = attrs.metadata();
            let mut extensions = span.extensions_mut();
            extensions.insert(EventKind(self.string_table.alloc(metadata.target())));
            let initial_kv = self.sid_from_metadata(metadata);
            let mut visitor = FieldVisitor(&self.string_table, initial_kv);
            attrs.record(&mut visitor);
            extensions.insert(SpanEventId(visitor.1));
        }
    }

    fn on_event(&self, event: &Event, _: Context<Subscriber>) {
        let metadata = event.metadata();
        let tid = thread_id::get() as u32;
        let name_id = self.string_table.alloc(metadata.target());
        let tstamp = self.nanos_since_start();
        let initial_kv = self.sid_from_metadata(metadata);
        let mut visitor = FieldVisitor(&self.string_table, initial_kv);
        event.record(&mut visitor);
        let event_id = EventId::from_u32(visitor.1.as_u32());
        let raw_event = RawEvent::new_instant(name_id, event_id, tid, tstamp);
        self.event_sink
            .write_atomic(std::mem::size_of::<RawEvent>(), move |bytes| {
                raw_event.serialize(bytes);
            });
    }

    fn on_record(&self, id: &Id, values: &Record, ctx: Context<Subscriber>) {
        // FIXME: this will add additional KV pairs to the string rather than replacing preexisting
        // one.
        if let Some(span) = ctx.span(id) {
            let mut extensions = span.extensions_mut();
            let old_kv = extensions
                .remove::<SpanEventId>()
                .map_or(StringId::INVALID, |x| x.0);
            let mut visitor = FieldVisitor(&self.string_table, old_kv);
            values.record(&mut visitor);
            extensions.replace(SpanEventId(visitor.1));
        }
    }

    fn on_enter(&self, id: &Id, ctx: Context<Subscriber>) {
        if let Some(span) = ctx.span(id) {
            let mut extensions = span.extensions_mut();
            // FIXME: this fails on a nested entry...
            extensions.insert(SpanTimestamp(self.nanos_since_start()));
        }
    }

    fn on_exit(&self, id: &Id, ctx: Context<Subscriber>) {
        if let Some(span) = ctx.span(id) {
            let now = self.nanos_since_start();
            let tid = thread_id::get() as u32;
            let mut extensions = span.extensions_mut();
            let event_kind = extensions
                .get_mut::<EventKind>()
                .map_or(StringId::INVALID, |x| x.0);
            let event_id = extensions
                .get_mut::<SpanEventId>()
                .map_or(StringId::INVALID, |x| x.0);
            let start = extensions.remove::<SpanTimestamp>().map_or(now, |x| x.0);
            drop(extensions);
            let event_id = EventId::from_u32(event_id.as_u32());
            let raw_event = RawEvent::new_interval(event_kind, event_id, tid, start, now);
            self.event_sink
                .write_atomic(std::mem::size_of::<RawEvent>(), move |bytes| {
                    raw_event.serialize(bytes);
                });
        }
    }
}

impl<'a, Sink: SerializationSink> Visit for FieldVisitor<'a, Sink> {
    fn record_debug(&mut self, field: &Field, value: &dyn std::fmt::Debug) {
        let label = field.name();
        let value = format!("={:?}", value);
        self.1 = self.0.alloc(&[
            StringComponent::Ref(self.1),
            StringComponent::Value(measureme::event_id::SEPARATOR_BYTE),
            StringComponent::Value(label),
            StringComponent::Value(&value),
        ]);
    }
}

#[cfg(test)]
mod test {
    use tracing::{
        dispatcher::{self, Dispatch},
        info, span, Level,
    };
    use tracing_subscriber::layer::SubscriberExt;

    fn new_subscriber(path: &str) -> tracing::subscriber::DefaultGuard {
        dispatcher::set_default(&Dispatch::new(tracing_subscriber::registry().with(
            super::MeasuremeLayer::<measureme::FileSerializationSink>::new(path).unwrap(),
        )))
    }

    #[test]
    fn it_works() {
        let _sub = new_subscriber("/tmp/tracing-measureme/it_works");
        {
            let span = span!(Level::TRACE, "banana", id = 42);
            let _enter = span.enter();
            span.record("id", &0);
            info!(message = "EXPLOSION!", banana = 42);
            info!("banana!!");
        }
    }

    #[test]
    fn out_of_order() {
        let _sub = new_subscriber("/tmp/tracing-measureme/out_of_order");
        let span1 = span!(Level::INFO, "out of order exits 1");
        let span2 = span!(Level::INFO, "out of order exits 2");
        let span3 = span!(Level::INFO, "out of order exits 3");
        let entry1 = span1.enter();
        let entry2 = span2.enter();
        let entry3 = span3.enter();
        drop(entry2);
        drop(entry3);
        drop(entry1);
    }

    #[test]
    fn multiple_entries() {
        let _sub = new_subscriber("/tmp/tracing-measureme/multiple-entries");
        let span = span!(Level::INFO, "multiple_entries");
        span.in_scope(|| {});
        span.in_scope(|| {});

        // FIXME: does not work
        // let span = span!(Level::INFO, "multiple_entries 2");
        // span.in_scope(|| {
        //     span.in_scope(|| {})
        // });
    }
}
