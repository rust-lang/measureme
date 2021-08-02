mod backwards_iter {
    // HACK(eddyb) like `DoubleEndedIterator`, but without a (forwards) `Iterator`.
    // This is needed because of how events are stored in "postorder",
    // i.e. an interval event follows all events nested in it, meaning
    // that most analyses we want to do can only be done in reverse.
    pub trait BackwardsIterator {
        type Item;
        fn next_back(&mut self) -> Option<Self::Item>;
    }

    pub struct Rev<I>(I);

    pub trait BackwardsIteratorExt: Sized {
        fn rev(self) -> Rev<Self>;
    }

    impl<I: BackwardsIterator> BackwardsIteratorExt for I {
        fn rev(self) -> Rev<Self> {
            Rev(self)
        }
    }

    impl<I: BackwardsIterator> Iterator for Rev<I> {
        type Item = I::Item;
        fn next(&mut self) -> Option<I::Item> {
            self.0.next_back()
        }
    }
}

use self::backwards_iter::{BackwardsIterator, BackwardsIteratorExt as _};
use analyzeme::{Event, EventPayload, ProfilingData, Timestamp};
use measureme::rustc::*;
use std::borrow::Cow;
use std::collections::BTreeMap;
use std::fmt;
use std::time::{Duration, SystemTime};

// FIXME(eddyb) move this into `analyzeme`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EventDescription<'a> {
    pub event_kind: Cow<'a, str>,
    pub label: Cow<'a, str>,
    pub additional_data: Vec<Cow<'a, str>>,
}

impl<'a> From<Event<'a>> for EventDescription<'a> {
    fn from(e: Event<'a>) -> Self {
        EventDescription {
            event_kind: e.event_kind,
            label: e.label,
            additional_data: e.additional_data,
        }
    }
}

impl fmt::Display for EventDescription<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.event_kind[..] {
            QUERY_EVENT_KIND | GENERIC_ACTIVITY_EVENT_KIND => {}
            _ => write!(f, "{} ", self.event_kind)?,
        }

        write!(f, "`{}(", self.label)?;
        for (i, arg) in self.additional_data.iter().enumerate() {
            if i > 0 {
                write!(f, ", ")?;
            }
            write!(f, "{}", arg)?;
        }
        write!(f, ")`")
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WithParent<T> {
    this: T,
    parent: Option<T>,
}

impl<'a> From<WithParent<Event<'a>>> for WithParent<EventDescription<'a>> {
    fn from(e: WithParent<Event<'a>>) -> Self {
        WithParent {
            this: e.this.into(),
            parent: e.parent.map(|e| e.into()),
        }
    }
}

// FIXME(eddyb) should all these variants have `E` in them? seems un-DRY
#[derive(Clone, Debug, PartialEq, Eq)]
enum SamplePoint<E> {
    Start(E),
    End(E),
    Instant(E),
}

impl<E> SamplePoint<E> {
    fn event(&self) -> &E {
        match self {
            SamplePoint::Start(event) | SamplePoint::End(event) | SamplePoint::Instant(event) => {
                event
            }
        }
    }

    fn map_event<E2>(self, f: impl FnOnce(E) -> E2) -> SamplePoint<E2> {
        match self {
            SamplePoint::Start(event) => SamplePoint::Start(f(event)),
            SamplePoint::End(event) => SamplePoint::End(f(event)),
            SamplePoint::Instant(event) => SamplePoint::Instant(f(event)),
        }
    }
}

impl SamplePoint<WithParent<Event<'_>>> {
    fn timestamp(&self) -> SystemTime {
        let timestamp = match self.event().this.payload {
            EventPayload::Timestamp(t) => t,
            _ => unreachable!(),
        };

        match (self, timestamp) {
            (SamplePoint::Start(_), Timestamp::Interval { start, .. }) => start,
            (SamplePoint::End(_), Timestamp::Interval { end, .. }) => end,
            (SamplePoint::Instant(_), Timestamp::Instant(time)) => time,
            _ => panic!(
                "SamplePoint::timestamp: event timestamp doesn't match \
                 `SamplePoint` variant, in `SamplePoint::{:?}`",
                self
            ),
        }
    }
}

struct SamplePoints<'a> {
    /// This analysis only works with deterministic runs, which precludes parallelism,
    /// so we just have to find the *only* thread's ID and require there is no other.
    expected_thread_id: u32,

    rev_events: std::iter::Peekable<Box<dyn Iterator<Item = Event<'a>> + 'a>>,
    stack: Vec<Event<'a>>,
}

impl<'a> SamplePoints<'a> {
    fn new<'b: 'a, I: Iterator<Item = Event<'a>> + DoubleEndedIterator + 'b>(events: I) -> Self {
        let mut rev_events = (Box::new(events.rev().filter(|e| !e.payload.is_integer()))
            as Box<dyn Iterator<Item = Event<'a>>>)
            .peekable();
        SamplePoints {
            // The `0` default doesn't matter, if there are no events.
            expected_thread_id: rev_events.peek().map_or(0, |event| event.thread_id),

            rev_events,
            stack: vec![],
        }
    }

    fn intervals(self) -> SampleIntervals<Self> {
        SampleIntervals::new(self)
    }
}

impl<'a> BackwardsIterator for SamplePoints<'a> {
    type Item = SamplePoint<WithParent<Event<'a>>>;
    fn next_back(&mut self) -> Option<Self::Item> {
        let sample_point = match self.rev_events.peek() {
            Some(peeked_event) if !peeked_event.payload.is_integer() => {
                assert_eq!(
                    peeked_event.thread_id, self.expected_thread_id,
                    "more than one thread is not supported in `summarize aggregate`"
                );
                match self.stack.last() {
                    // Make sure to first leave any events in the stack that succeed
                    // this one (note that because we're `peek`-ing, this will keep
                    // getting hit until we run out of stack entries to leave).
                    Some(top_event) if !top_event.contains(peeked_event) => {
                        SamplePoint::Start(self.stack.pop().unwrap())
                    }
                    Some(_) => unreachable!(),

                    _ => {
                        let event = self.rev_events.next().unwrap();
                        match event.payload {
                            EventPayload::Timestamp(Timestamp::Interval { .. }) => {
                                // Now entering this new event.
                                self.stack.push(event.clone());
                                SamplePoint::End(event)
                            }

                            EventPayload::Timestamp(Timestamp::Instant(_)) => {
                                SamplePoint::Instant(event)
                            }
                            EventPayload::Integer(_) => {
                                unreachable!()
                            }
                        }
                    }
                }
            }
            Some(_) => unreachable!(),

            // Ran out of events, but we might still have stack entries to leave.
            None => SamplePoint::Start(self.stack.pop()?),
        };

        // HACK(eddyb) this works around `SamplePoint::End` having pushed itself
        // onto the stack, so its parent isn't the top of the stack anymore.
        let parent = match sample_point {
            SamplePoint::End(_) => {
                if self.stack.len() >= 2 {
                    Some(&self.stack[self.stack.len() - 2])
                } else {
                    None
                }
            }
            SamplePoint::Start(_) | SamplePoint::Instant(_) => self.stack.last(),
        };

        Some(sample_point.map_event(|this| WithParent {
            this,
            parent: parent.cloned(),
        }))
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct SampleInterval<E> {
    start: SamplePoint<E>,
    end: SamplePoint<E>,
}

impl<E> SampleInterval<E> {
    fn map_event<E2>(self, f: impl Copy + FnOnce(E) -> E2) -> SampleInterval<E2> {
        SampleInterval {
            start: self.start.map_event(f),
            end: self.end.map_event(f),
        }
    }
}

impl SampleInterval<WithParent<Event<'_>>> {
    fn duration(&self) -> Duration {
        self.end
            .timestamp()
            .duration_since(self.start.timestamp())
            .unwrap()
    }
}

struct SampleIntervals<I: BackwardsIterator> {
    last_sample_point: Option<I::Item>,

    sample_points: I,
}

impl<I: BackwardsIterator> SampleIntervals<I> {
    fn new(mut sample_points: I) -> Self {
        SampleIntervals {
            last_sample_point: sample_points.next_back(),

            sample_points,
        }
    }
}

impl<E: Clone, I: BackwardsIterator<Item = SamplePoint<E>>> BackwardsIterator
    for SampleIntervals<I>
{
    type Item = SampleInterval<E>;
    fn next_back(&mut self) -> Option<Self::Item> {
        let start = self.sample_points.next_back()?;
        // FIXME(eddyb) make this cloning cheaper (somehow?)
        let end = self.last_sample_point.replace(start.clone())?;

        Some(SampleInterval { start, end })
    }
}

// FIXME(eddyb) extend this with more statistical information, rather
// than assuming uniform distribution inside the range (`min..=max`).
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
struct Variance<T> {
    /// The size of the range of possible values, i.e. `max - min`.
    range_size: T,
}

struct AggregatedSampleInterval<'a> {
    descriptions: SampleInterval<WithParent<EventDescription<'a>>>,

    min_duration: Duration,
    duration_variance: Variance<Duration>,
}

impl AggregatedSampleInterval<'_> {
    fn max_duration(&self) -> Duration {
        self.min_duration + self.duration_variance.range_size
    }
}

struct AggregatedSampleIntervals<I> {
    sample_intervals_per_profile: Vec<I>,
}

impl<'a, I: BackwardsIterator<Item = SampleInterval<WithParent<Event<'a>>>>>
    AggregatedSampleIntervals<I>
{
    fn new(sample_intervals_per_profile: impl Iterator<Item = I>) -> Self {
        AggregatedSampleIntervals {
            sample_intervals_per_profile: sample_intervals_per_profile.collect(),
        }
    }
}

impl<'a, I: BackwardsIterator<Item = SampleInterval<WithParent<Event<'a>>>>> BackwardsIterator
    for AggregatedSampleIntervals<I>
{
    type Item = AggregatedSampleInterval<'a>;
    fn next_back(&mut self) -> Option<Self::Item> {
        match self.sample_intervals_per_profile.get_mut(0)?.next_back() {
            Some(interval) => {
                let first_duration = interval.duration();
                let descriptions = interval.map_event(WithParent::<EventDescription>::from);

                // FIXME(eddyb) maybe extract this part into an `Iterator` impl? but it
                // would be hard to return an interable that doesn't allocate nor borrow
                // the iterator (whereas here `durations_across_profiles` borrows
                // `self.sample_intervals_per_profile`)
                let mut durations_across_profiles = std::iter::once(first_duration).chain(
                    self.sample_intervals_per_profile[1..].iter_mut().map(|it| {
                        let interval = it
                            .next_back()
                            .expect("`summarize aggregate` requires identical sequences of events");

                        let duration = interval.duration();

                        // Ensure we don't allow profiles that differ in event details.
                        // FIXME(eddyb) this may be expensive (and is redundant
                        // for every event, shared by adjacent intervals), there
                        // should be a cheaper way to compare strings across
                        // string tables, or even enforce that the string tables
                        // of each profile are themselves identical.
                        assert_eq!(
                            descriptions,
                            interval.map_event(WithParent::<EventDescription>::from),
                            "`summarize aggregate` requires identical sequences of events"
                        );

                        duration
                    }),
                );

                let (mut min_duration, mut max_duration) = {
                    let first = durations_across_profiles.next().unwrap();
                    (first, first)
                };
                for duration in durations_across_profiles {
                    min_duration = min_duration.min(duration);
                    max_duration = max_duration.max(duration);
                }

                Some(AggregatedSampleInterval {
                    descriptions,

                    min_duration,
                    duration_variance: Variance {
                        range_size: max_duration - min_duration,
                    },
                })
            }
            None => {
                for leftover_intervals in self.sample_intervals_per_profile.iter_mut() {
                    assert_eq!(
                        leftover_intervals.next_back(),
                        None,
                        "`summarize aggregate` requires identical sequences of events"
                    );
                }
                None
            }
        }
    }
}

// FIXME(eddyb) move this somewhere else
// (counterpoint: tracking "sources" of values is too specific)
pub struct Extrema<T, S = ()> {
    /// Number of `smallest`/`largest` values to keep track of.
    limit: usize,

    pub smallest: BTreeMap<T, ExtremaSources<S>>,
    pub largest: BTreeMap<T, ExtremaSources<S>>,
}

pub enum ExtremaSources<S> {
    Empty,
    One(S),
    Count(usize),
}

impl<S> Default for ExtremaSources<S> {
    fn default() -> Self {
        ExtremaSources::Empty
    }
}

impl<S: Clone> ExtremaSources<S> {
    pub fn count(&self) -> usize {
        match *self {
            ExtremaSources::Empty => 0,
            ExtremaSources::One(_) => 1,
            ExtremaSources::Count(count) => count,
        }
    }

    pub fn add(&mut self, source: &S) {
        *self = match self {
            ExtremaSources::Empty => ExtremaSources::One(source.clone()),
            _ => ExtremaSources::Count(self.count() + 1),
        };
    }
}

impl<T: Copy + Ord, S: Clone> Extrema<T, S> {
    pub fn new(limit: usize) -> Self {
        Extrema {
            limit,

            smallest: BTreeMap::new(),
            largest: BTreeMap::new(),
        }
    }

    pub fn add(&mut self, value: T, source: &S) {
        self.add_range(value..=value, source)
    }

    pub fn add_range(&mut self, range: std::ops::RangeInclusive<T>, source: &S) {
        enum Which {
            Smallest,
            Largest,
        }

        for which in &[Which::Smallest, Which::Largest] {
            let (map, &value) = match which {
                Which::Smallest => (&mut self.smallest, range.start()),
                Which::Largest => (&mut self.largest, range.end()),
            };
            if map.len() < self.limit {
                map.entry(value).or_default().add(source);
            } else {
                let least_extreme = match which {
                    Which::Smallest => map.keys().rev().next().copied().unwrap(), // `max(smallest)`
                    Which::Largest => map.keys().next().copied().unwrap(),        // `min(largest)`
                };
                let less_extreme = match which {
                    Which::Smallest => value > least_extreme, // `value > max(smallest)`
                    Which::Largest => value < least_extreme,  // `value < min(largest)`
                };
                if !less_extreme {
                    map.entry(value).or_default().add(source);

                    if map.len() > self.limit {
                        map.remove(&least_extreme);
                    }

                    assert_eq!(map.len(), self.limit);
                }
            }
        }
    }
}

pub fn aggregate_profiles(profiles: Vec<ProfilingData>) {
    let aggregated_sample_intervals = AggregatedSampleIntervals::new(
        profiles
            .iter()
            .map(|data| SamplePoints::new(data.iter().map(|event| event.to_event())).intervals()),
    );

    let mut intervals_count = 0;

    // FIXME(eddyb) make the `10` configurable at runtime (i.e. with a flag)
    let mut durations = Extrema::new(10);
    let mut variances = Extrema::new(10);

    for interval in aggregated_sample_intervals.rev() {
        intervals_count += 1;

        durations.add_range(
            interval.min_duration..=interval.max_duration(),
            &interval.descriptions,
        );
        variances.add(interval.duration_variance, &interval.descriptions);
    }

    let describe =
        |descriptions: ExtremaSources<SampleInterval<WithParent<EventDescription<'_>>>>| {
            if let ExtremaSources::One(description) = descriptions {
                match (description.start, description.end) {
                    (SamplePoint::Start(start), SamplePoint::End(end)) => {
                        assert_eq!(start, end);
                        start.this.to_string()
                    }

                    (SamplePoint::Start(outer), SamplePoint::Start(inner))
                    | (SamplePoint::Start(outer), SamplePoint::Instant(inner)) => {
                        assert_eq!(inner.parent.as_ref(), Some(&outer.this));
                        format!("in {}, before {}", outer.this, inner.this)
                    }

                    (SamplePoint::End(inner), SamplePoint::End(outer))
                    | (SamplePoint::Instant(inner), SamplePoint::End(outer)) => {
                        assert_eq!(inner.parent.as_ref(), Some(&outer.this));
                        format!("in {}, after {}", outer.this, inner.this)
                    }

                    (SamplePoint::End(first), SamplePoint::Start(second))
                    | (SamplePoint::Instant(first), SamplePoint::Start(second))
                    | (SamplePoint::End(first), SamplePoint::Instant(second))
                    | (SamplePoint::Instant(first), SamplePoint::Instant(second)) => {
                        assert_eq!(first.parent, second.parent);
                        if let Some(parent) = &first.parent {
                            format!(
                                "in {},\n    between {}\n        and {}\n",
                                parent, first.this, second.this
                            )
                        } else {
                            format!("between {} and {}", first.this, second.this)
                        }
                    }
                }
            } else {
                let count = descriptions.count();
                format!(
                    "{} occurrences, or {:.2}%",
                    count,
                    (count as f64) / (intervals_count as f64) * 100.0
                )
            }
        };

    println!("Smallest {} durations:", durations.smallest.len());
    for (duration, descriptions) in durations.smallest {
        println!("  {} ns: {}", duration.as_nanos(), describe(descriptions));
    }
    println!("");
    println!("Largest {} durations:", durations.largest.len());
    for (duration, descriptions) in durations.largest {
        println!("  {} ns: {}", duration.as_nanos(), describe(descriptions));
    }
    println!("");
    println!("Smallest {} variances:", variances.smallest.len());
    for (variance, descriptions) in variances.smallest {
        println!(
            "  ±{} ns: {}",
            variance.range_size.as_nanos() as f64 / 2.0,
            describe(descriptions)
        );
    }
    println!();
    println!("Largest {} variances:", variances.largest.len());
    for (variance, descriptions) in variances.largest {
        println!(
            "  ±{} ns: {}",
            variance.range_size.as_nanos() as f64 / 2.0,
            describe(descriptions)
        );
    }
}
