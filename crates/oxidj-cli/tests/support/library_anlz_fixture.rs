fn be(v: u32) -> [u8; 4] {
    v.to_be_bytes()
}

fn anlz_section(tag: &[u8; 4], header_rest: &[u8], body: &[u8]) -> Vec<u8> {
    let header = 12 + header_rest.len() as u32;
    [
        &tag[..],
        &be(header),
        &be(header + body.len() as u32),
        header_rest,
        body,
    ]
    .concat()
}

fn anlz_file(sections: &[Vec<u8>]) -> Vec<u8> {
    let body = sections.concat();
    [
        &b"PMAI"[..],
        &be(28),
        &be(28 + body.len() as u32),
        &[0; 16],
        &body,
    ]
    .concat()
}

fn utf16(text: &str) -> Vec<u8> {
    text.encode_utf16()
        .chain([0])
        .flat_map(u16::to_be_bytes)
        .collect()
}

fn anlz_path(path: &str) -> Vec<u8> {
    let body = utf16(path);
    anlz_section(b"PPTH", &be(body.len() as u32), &body)
}

/// (beat 1..=4, BPM×100, time ms).
fn anlz_grid(beats: &[(u16, u16, u32)]) -> Vec<u8> {
    let rest = [be(0), be(0x0008_0000), be(beats.len() as u32)].concat();
    let body: Vec<u8> = beats
        .iter()
        .flat_map(|(b, t, ms)| [&b.to_be_bytes()[..], &t.to_be_bytes(), &be(*ms)].concat())
        .collect();
    anlz_section(b"PQTZ", &rest, &body)
}

/// PCOB list of PCPT entries (hot, type 1 point / 2 loop, time ms, loop end ms).
fn anlz_cues(list: u32, cues: &[(u32, u8, u32, u32)]) -> Vec<u8> {
    let entries: Vec<u8> = cues
        .iter()
        .flat_map(|&(hot, kind, ms, end)| {
            let mut e = [
                &b"PCPT"[..],
                &be(28),
                &be(56),
                &be(hot),
                &be(0),
                &be(0x10000),
            ]
            .concat();
            e.extend([0xff, 0xff, 0xff, 0xff, kind, 0, 0x03, 0xe8]);
            e.extend(be(ms));
            e.extend(be(end));
            e.resize(56, 0);
            e
        })
        .collect();
    let rest = [
        &be(list)[..],
        &0u16.to_be_bytes(),
        &(cues.len() as u16).to_be_bytes(),
        &be(u32::MAX),
    ]
    .concat();
    anlz_section(b"PCOB", &rest, &entries)
}

