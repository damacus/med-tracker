use std::sync::{Arc, Mutex};
use tracing::{
    Event, Metadata, Subscriber,
    field::{Field, Visit},
    span::{Attributes, Id, Record},
};

#[derive(Clone, Default)]
pub struct Captured(Arc<Mutex<State>>);

#[derive(Default)]
struct State {
    spans: Vec<String>,
    entered: Vec<usize>,
    events: Vec<String>,
}

#[derive(Default)]
struct Fields(String);

impl Visit for Fields {
    fn record_debug(&mut self, field: &Field, value: &dyn std::fmt::Debug) {
        use std::fmt::Write;
        write!(self.0, " {}={value:?}", field.name()).expect("write captured field");
    }
}

impl Captured {
    pub fn events(&self) -> Vec<String> {
        self.0.lock().expect("trace capture lock").events.clone()
    }
}

impl Subscriber for Captured {
    fn enabled(&self, _: &Metadata<'_>) -> bool {
        true
    }

    fn new_span(&self, attributes: &Attributes<'_>) -> Id {
        let mut fields = Fields::default();
        attributes.record(&mut fields);
        let mut state = self.0.lock().expect("trace capture lock");
        state.spans.push(fields.0);
        Id::from_u64(u64::try_from(state.spans.len()).expect("span count"))
    }

    fn record(&self, _: &Id, _: &Record<'_>) {}

    fn record_follows_from(&self, _: &Id, _: &Id) {}

    fn event(&self, event: &Event<'_>) {
        let mut fields = Fields::default();
        event.record(&mut fields);
        let mut state = self.0.lock().expect("trace capture lock");
        for index in &state.entered {
            if let Some(span) = state.spans.get(*index) {
                fields.0.push_str(span);
            }
        }
        state.events.push(fields.0);
    }

    fn enter(&self, id: &Id) {
        self.0
            .lock()
            .expect("trace capture lock")
            .entered
            .push(usize::try_from(id.into_u64() - 1).expect("span index"));
    }

    fn exit(&self, id: &Id) {
        let index = self.0.lock().expect("trace capture lock").entered.pop();
        assert_eq!(
            index,
            Some(usize::try_from(id.into_u64() - 1).expect("span index"))
        );
    }
}
