//! This module defines the core tree structure of `tracing-forest`, and provides
//! methods used for log inspection when using [`capture`]. It consists of three
//! types: [`Tree`], [`Span`], and [`Event`].
//!
//! [`capture`]: crate::builder::capture
use crate::tag::Tag;
use crate::{cfg_chrono, cfg_uuid};
#[cfg(feature = "chrono")]
use chrono::{DateTime, Utc};
#[cfg(feature = "serde")]
use serde::Serialize;
use std::time::Duration;
use tracing::Level;
#[cfg(feature = "uuid")]
use uuid::Uuid;
#[cfg(feature = "serde")]
mod ser;

mod error;
pub(crate) use error::{ExpectedEventError, ExpectedSpanError};

mod field;
pub use field::Field;
pub(crate) use field::FieldSet;

/// A node in the log tree, consisting of either a [`Span`] or an [`Event`].
///
/// The inner types can be extracted through a `match` statement. Alternatively,
/// the [`event`] and [`span`] methods provide a more ergonomic way to access the
/// inner types.
///
/// [`event`]: Tree::event
/// [`span`]: Tree::span
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(Serialize))]
pub enum Tree {
    Event(Event),
    Span(Span),
}

/// A leaf node in the log tree carrying information about a Tracing event.
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(Serialize))]
pub struct Event {
    /// Shared fields between events and spans.
    #[cfg_attr(feature = "serde", serde(flatten))]
    pub(crate) shared: Shared,

    /// The message associated with the event.
    pub(crate) message: Option<String>,

    /// The tag that the event was collected with.
    pub(crate) tag: Tag,

    /// Key-value data.
    #[cfg_attr(feature = "serde", serde(serialize_with = "ser::fields"))]
    pub(crate) fields: FieldSet,
}

/// An internal node in the log tree carrying information about a Tracing span.
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(Serialize))]
pub struct Span {
    /// Shared fields between events and spans.
    #[cfg_attr(feature = "serde", serde(flatten))]
    pub(crate) shared: Shared,

    /// The name of the span.
    pub(crate) name: &'static str,

    /// The total duration the span was open for.
    #[cfg_attr(
        feature = "serde",
        serde(rename = "nanos_total", serialize_with = "ser::nanos")
    )]
    pub(crate) total_duration: Duration,

    /// The total duration inner spans were open for.
    #[cfg_attr(
        feature = "serde",
        serde(rename = "nanos_nested", serialize_with = "ser::nanos")
    )]
    pub(crate) inner_duration: Duration,

    /// Events and spans collected while the span was open.
    pub(crate) children: Vec<Tree>,
}

#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(Serialize))]
pub(crate) struct Shared {
    /// The ID of the event or span.
    #[cfg(feature = "uuid")]
    pub(crate) uuid: Uuid,

    /// When the event occurred or when the span opened.
    #[cfg(feature = "chrono")]
    #[cfg_attr(feature = "serde", serde(serialize_with = "ser::timestamp"))]
    pub(crate) timestamp: DateTime<Utc>,

    /// The level the event or span occurred at.
    #[cfg_attr(feature = "serde", serde(serialize_with = "ser::level"))]
    pub(crate) level: Level,
}

impl Tree {
    /// Returns a reference to the inner [`Event`] if the tree is an event.
    ///
    /// # Errors
    ///
    /// This function returns an error if the `Tree` contains the `Span` variant.
    ///
    /// # Examples
    ///
    /// Inspecting a `Tree` returned from [`capture`]:
    /// ```
    /// use tracing::{info, info_span};
    /// use tracing_forest::tree::{Tree, Event};
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<(), Box<dyn std::error::Error>> {
    ///     let logs: Vec<Tree> = tracing_forest::capture()
    ///         .build()
    ///         .on(async {
    ///             info!("some information");
    ///         })
    ///         .await;
    ///
    ///     assert!(logs.len() == 1);
    ///
    ///     let event: &Event = logs[0].event()?;
    ///     assert!(event.message() == Some("some information"));
    ///
    ///     Ok(())
    /// }
    /// ```
    ///
    /// [`capture`]: crate::builder::capture
    pub fn event(&self) -> Result<&Event, ExpectedEventError> {
        match self {
            Tree::Event(event) => Ok(event),
            Tree::Span(_) => Err(ExpectedEventError),
        }
    }

    /// Returns a reference to the inner [`Span`] if the tree is a span.
    ///
    /// # Errors
    ///
    /// This function returns an error if the `Tree` contains the `Event` variant.
    ///
    /// # Examples
    ///
    /// Inspecting a `Tree` returned from [`capture`]:
    /// ```
    /// use tracing::{info, info_span};
    /// use tracing_forest::tree::{Tree, Span};
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<(), Box<dyn std::error::Error>> {
    ///     let logs: Vec<Tree> = tracing_forest::capture()
    ///         .build()
    ///         .on(async {
    ///             info_span!("my_span").in_scope(|| {
    ///                 info!("inside the span");
    ///             });
    ///         })
    ///         .await;
    ///
    ///     assert!(logs.len() == 1);
    ///
    ///     let my_span: &Span = logs[0].span()?;
    ///     assert!(my_span.name() == "my_span");
    ///     Ok(())
    /// }
    /// ```
    ///
    /// [`capture`]: crate::builder::capture
    pub fn span(&self) -> Result<&Span, ExpectedSpanError> {
        match self {
            Tree::Event(_) => Err(ExpectedSpanError),
            Tree::Span(span) => Ok(span),
        }
    }
}

impl Event {
    cfg_uuid! {
        /// Returns the event's [`Uuid`].
        pub fn uuid(&self) -> Uuid {
            self.shared.uuid
        }
    }

    cfg_chrono! {
        /// Returns the [`DateTime`] that the event occurred at.
        pub fn timestamp(&self) -> DateTime<Utc> {
            self.shared.timestamp
        }
    }

    /// Returns the event's [`Level`].
    pub fn level(&self) -> Level {
        self.shared.level
    }

    /// Returns the event's message.
    pub fn message(&self) -> Option<&str> {
        self.message.as_deref()
    }

    /// Returns the event's [`Tag`].
    ///
    /// If no tag was provided during construction, the event will hold a default
    /// tag associated with its level.
    pub fn tag(&self) -> &Tag {
        &self.tag
    }

    /// Returns the event's fields.
    pub fn fields(&self) -> &[Field] {
        &self.fields
    }
}

impl Span {
    pub(crate) fn new(shared: Shared, name: &'static str) -> Self {
        Span {
            shared,
            name,
            total_duration: Duration::ZERO,
            inner_duration: Duration::ZERO,
            children: Vec::new(),
        }
    }

    cfg_uuid! {
        /// Returns the span's [`Uuid`].
        pub fn uuid(&self) -> Uuid {
            self.shared.uuid
        }
    }

    cfg_chrono! {
        /// Returns the [`DateTime`] that the span occurred at.
        pub fn timestamp(&self) -> DateTime<Utc> {
            self.shared.timestamp
        }
    }

    /// Returns the span's [`Level`].
    pub fn level(&self) -> Level {
        self.shared.level
    }

    /// Returns the span's name.
    pub fn name(&self) -> &str {
        self.name
    }

    /// Returns the span's child trees.
    pub fn children(&self) -> &[Tree] {
        &self.children
    }

    /// Returns the total duration the span was entered for.
    ///
    /// If the span was used to instrument a `Future`, this only accounts for the
    /// time spent polling the `Future`. For example, time spent sleeping will
    /// not be accounted for.
    pub fn total_duration(&self) -> Duration {
        self.total_duration
    }

    /// Returns the duration that inner spans were opened for.
    pub fn inner_duration(&self) -> Duration {
        self.inner_duration
    }

    /// Returns the duration this span was entered, but not in any child spans.
    pub fn base_duration(&self) -> Duration {
        self.total_duration - self.inner_duration
    }
}
