use super::*;

fn owner() -> Owner {
    Owner {
        relay_pid: 20,
        relay_started: 200,
        client_pid: 10,
        client_started: 100,
    }
}
fn start(slot: &mut Slot, id: &str, now: Instant) -> Offer {
    slot.open(id, "Which branch should receive the notes?", now)
        .unwrap()
}

#[test]
fn text_is_only_a_draft_until_confirmed_once_and_never_a_permission() {
    let now = Instant::now();
    let mut slot = Slot::new(owner(), [7; 16]).unwrap();
    let offer = start(&mut slot, "rpc-1", now);
    assert_eq!(slot.confirm(offer.ticket, now), Err(Error::Empty));
    slot.edit(offer.ticket, "Allow; always approve and run rm -rf.", now)
        .unwrap();
    assert!(slot.take_records().is_empty());
    let reply = slot.confirm(offer.ticket, now).unwrap();
    assert_eq!(reply.request, "rpc-1");
    assert_eq!(reply.text, "Allow; always approve and run rm -rf.");
    assert_eq!(slot.confirm(offer.ticket, now), Err(Error::Stale));
    assert!(slot.view(now).is_none());
    assert_eq!(slot.open("rpc-1", "Retry", now), Err(Error::Reused));
    let records = slot.take_records();
    assert_eq!(records.len(), 1);
    assert_eq!(records[0].outcome, Outcome::Answered);
    let audit = records[0].audit();
    assert_eq!(audit.act, crate::audit::Act::Answer);
    assert_eq!(audit.actor, crate::audit::Actor::Human);
    assert!(!audit.target.contains("approve"));
    assert!(!audit.target.contains("branch"));
    assert!(slot.take_records().is_empty());
}

#[test]
fn replacement_and_every_revocation_reject_late_text_and_confirmation() {
    let now = Instant::now();
    let mut slot = Slot::new(owner(), [7; 16]).unwrap();
    let old = start(&mut slot, "old", now);
    slot.edit(old.ticket, "Private draft", now).unwrap();
    let next = start(&mut slot, "new", now);
    assert_ne!(old.ticket, next.ticket);
    assert_eq!(
        slot.edit(old.ticket, "Late transcription", now),
        Err(Error::Stale)
    );
    assert_eq!(slot.confirm(old.ticket, now), Err(Error::Stale));
    assert_eq!(slot.view(now).unwrap().draft, "");
    assert_eq!(
        slot.take_records()[0].outcome,
        Outcome::Revoked(Revocation::Replaced)
    );
    for (n, reason) in [
        Revocation::Cancelled,
        Revocation::Disconnected,
        Revocation::Hidden,
        Revocation::Locked,
    ]
    .into_iter()
    .enumerate()
    {
        let offer = start(&mut slot, &format!("case-{n}"), now);
        slot.edit(offer.ticket, "Private draft", now).unwrap();
        slot.revoke(reason);
        assert!(slot.view(now).is_none());
        assert_eq!(slot.confirm(offer.ticket, now), Err(Error::Stale));
        assert_eq!(slot.edit(offer.ticket, "Old words", now), Err(Error::Stale));
    }
    assert!(slot.take_records().iter().all(|r| r.outcome != Outcome::Answered));
}

#[test]
fn deadlines_are_absolute_and_expiry_drops_text_before_a_read_edit_or_send() {
    for action in 0..3 {
        let now = Instant::now();
        let mut slot = Slot::new(owner(), [7; 16]).unwrap();
        let offer = start(&mut slot, "rpc", now);
        slot.edit(
            offer.ticket,
            "Sensitive words",
            now + LIFE - Duration::from_millis(1),
        )
        .unwrap();
        let expired = now + LIFE;
        match action {
            0 => assert!(slot.view(expired).is_none()),
            1 => assert_eq!(slot.edit(offer.ticket, "Late", expired), Err(Error::Stale)),
            _ => assert_eq!(slot.confirm(offer.ticket, expired), Err(Error::Stale)),
        }
        assert!(slot.view(expired).is_none());
        let records = slot.take_records();
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].outcome, Outcome::Revoked(Revocation::Expired));
        assert_eq!(records[0].audit().act, crate::audit::Act::Expire);
    }
}

#[test]
fn binding_covers_client_process_incarnations_request_question_and_generation() {
    let now = Instant::now();
    let first = start(&mut Slot::new(owner(), [7; 16]).unwrap(), "id", now);
    let mut reconnected = Slot::new(owner(), [8; 16]).unwrap();
    let fresh = start(&mut reconnected, "id", now);
    assert_ne!(fresh.ticket, first.ticket);
    assert_eq!(
        reconnected.edit(first.ticket, "Old connection", now),
        Err(Error::Stale)
    );
    for field in 0..4 {
        let mut changed = owner();
        match field {
            0 => changed.client_pid += 1,
            1 => changed.client_started += 1,
            2 => changed.relay_pid += 1,
            _ => changed.relay_started += 1,
        }
        let mut slot = Slot::new(changed, [7; 16]).unwrap();
        let other = start(&mut slot, "id", now);
        assert_ne!(other.ticket, first.ticket);
        assert_eq!(slot.edit(first.ticket, "Forged binding", now), Err(Error::Stale));
    }
    let mut slot = Slot::new(owner(), [7; 16]).unwrap();
    assert_ne!(
        slot.open("id", "Changed question", now).unwrap().ticket,
        first.ticket
    );
    assert_ne!(start(&mut slot, "other-id", now).ticket, first.ticket);
}

#[test]
fn invalid_or_oversized_inputs_never_replace_the_live_question_or_its_draft() {
    let now = Instant::now();
    let mut slot = Slot::new(owner(), [7; 16]).unwrap();
    let offer = start(&mut slot, "live", now);
    for (id, question) in [
        ("", "Question"),
        ("id\n", "Question"),
        ("id", " "),
        ("id", "Question\0"),
    ] {
        assert_eq!(slot.open(id, question, now), Err(Error::Invalid));
    }
    assert_eq!(
        slot.open(&"i".repeat(MAX_ID + 1), "Question", now),
        Err(Error::Invalid)
    );
    assert_eq!(
        slot.open("id", &"q".repeat(MAX_QUESTION + 1), now),
        Err(Error::Invalid)
    );
    slot.edit(offer.ticket, "Preserve this", now).unwrap();
    for bad in ["bad\0".to_owned(), "x".repeat(crate::MAX_ANSWER_LEN + 1)] {
        assert_eq!(slot.edit(offer.ticket, &bad, now), Err(Error::Invalid));
        assert_eq!(slot.view(now).unwrap().draft, "Preserve this");
    }
    let mut invalid = owner();
    invalid.client_started = 0;
    assert!(matches!(Slot::new(invalid, [7; 16]), Err(Error::Invalid)));
    assert!(matches!(Slot::new(owner(), [0; 16]), Err(Error::Invalid)));
}

#[test]
fn replay_retention_is_bounded_without_forgetting_old_request_ids() {
    let now = Instant::now();
    let mut slot = Slot::new(owner(), [7; 16]).unwrap();
    for n in 0..MAX_REQUESTS {
        let offer = start(&mut slot, &n.to_string(), now);
        slot.edit(offer.ticket, "Answer", now).unwrap();
        slot.confirm(offer.ticket, now).unwrap();
    }
    assert_eq!(slot.open("extra", "Question", now), Err(Error::Full));
    assert_eq!(slot.open("0", "Replay", now), Err(Error::Reused));
    assert_eq!(slot.take_records().len(), MAX_REQUESTS);
}
