use provider_mapping::{Direction, Mapping, Registry, Status};
use serde_json::{json, Value};

const IMAGE: &[u8] = include_bytes!("../../../../examples/mapping/example-image.map.json");

fn example() -> Mapping {
    Mapping::parse(IMAGE).unwrap()
}

#[test]
fn image_maps_request_and_provider_result_and_requires_approval() {
    let registry = Registry::builtin().unwrap();
    let mut mapping = example();
    let canonical: Value =
        serde_json::from_str(include_str!("../../../../examples/mapping/request.json")).unwrap();
    let upstream: Value =
        serde_json::from_str(include_str!("../../../../examples/mapping/response.json")).unwrap();

    assert_eq!(
        mapping
            .apply(
                &registry,
                "self_hosted",
                "example-image",
                Direction::Request,
                &canonical
            )
            .unwrap(),
        json!({"description":"Two colored samples","outputs":2,"layout":"Landscape"})
    );
    assert_eq!(
        mapping
            .preview(&registry, Direction::Response, &upstream)
            .unwrap()["images"][0],
        json!({"url":"https://files.example.test/first.png","content_type":"image/png"})
    );
    assert!(mapping
        .apply(
            &registry,
            "other",
            "example-image",
            Direction::Request,
            &canonical
        )
        .is_err());
    mapping.status = Status::Draft;
    assert!(mapping
        .apply(
            &registry,
            "self_hosted",
            "example-image",
            Direction::Request,
            &canonical
        )
        .is_err());
    assert!(mapping
        .preview(&registry, Direction::Request, &canonical)
        .is_ok());
    let mut unsupported = canonical;
    unsupported["aspect_ratio"] = json!("16:9");
    assert!(mapping
        .preview(&registry, Direction::Request, &unsupported)
        .is_err());
}

#[test]
fn every_shared_primitive_can_map_a_payload_and_result() {
    let registry = Registry::builtin().unwrap();
    for (primitive, request, result_field, result) in [
        (
            "text",
            json!({"model":"one","messages":[{"role":"user","content":"hello"}],"max_tokens":16,"stream":false}),
            "text",
            json!("hello"),
        ),
        (
            "image",
            json!({"kind":"image","prompt":"a","count":1,"aspect_ratio":"1:1"}),
            "images",
            json!([{"url":"output.png"}]),
        ),
        (
            "video",
            json!({"kind":"video","prompt":"a","duration_seconds":5,"aspect_ratio":"16:9","resolution":"720p"}),
            "video",
            json!({"url":"output.mp4"}),
        ),
        (
            "audio",
            json!({"kind":"audio","prompt":"a","duration_seconds":5,"format":"wav"}),
            "audio",
            json!({"url":"output.wav"}),
        ),
        (
            "mesh",
            json!({"kind":"mesh","prompt":"a","format":"glb"}),
            "mesh",
            json!({"url":"output.glb"}),
        ),
        (
            "world",
            json!({"task":"text_to_world","prompt":"a"}),
            "world",
            json!({"url":"output.spz","format":"spz"}),
        ),
    ] {
        let rules = json!({
            "version":1, "primitive":primitive, "schema":registry.id(), "provider":"custom",
            "model":"one", "revision":"v1", "status":"approved", "rules":{
                "request":[{"to":"/payload","value":{"op":"copy","from":""}}],
                "response":[{"to":format!("/{result_field}"),"value":{"op":"copy","from":"/data"}}]
            }
        });
        let mapping = Mapping::parse(&serde_json::to_vec(&rules).unwrap()).unwrap();
        assert_eq!(
            mapping
                .preview(&registry, Direction::Request, &request)
                .unwrap()["payload"],
            request
        );
        assert_eq!(
            mapping
                .preview(&registry, Direction::Response, &json!({"data": result}))
                .unwrap()[result_field],
            result
        );
    }
}

#[test]
fn rejected_conflicts_and_unknown_paths_prevent_silent_rewrites() {
    let registry = Registry::builtin().unwrap();
    let mut edited: Value = serde_json::from_slice(IMAGE).unwrap();
    edited["rules"]["request"][1]["value"]["from"] = json!("/amount");
    assert!(Mapping::parse(&serde_json::to_vec(&edited).unwrap())
        .unwrap()
        .validate(&registry)
        .is_err());
    edited["rules"]["request"][1]["value"]["from"] = json!("/count");
    edited["rules"]["request"][1]["to"] = json!("/description/detail");
    assert!(Mapping::parse(&serde_json::to_vec(&edited).unwrap())
        .unwrap()
        .validate(&registry)
        .is_err());
    edited["rules"]["request"][1]["to"] = json!("/outputs");
    edited["unexpected"] = json!("run this");
    assert!(Mapping::parse(&serde_json::to_vec(&edited).unwrap()).is_err());
}

#[test]
fn new_standard_fields_work_with_an_explicit_registry_and_mapping() {
    let registry = Registry::builtin().unwrap();
    let mut amended = registry.document().clone();
    amended["$id"] = json!("urn:tokencraft:primitives:custom-1");
    amended["$defs"]["image_input"]["properties"]["seed"] = json!({"type":"integer","minimum":0});
    let local = Registry::from_slice(&serde_json::to_vec(&amended).unwrap()).unwrap();
    let mut file: Value = serde_json::from_slice(IMAGE).unwrap();
    file["schema"] = json!(local.id());
    let request = json!({"kind":"image","prompt":"test","count":1,"aspect_ratio":"1:1","seed":12});
    let mapping = Mapping::parse(&serde_json::to_vec(&file).unwrap()).unwrap();
    assert!(mapping.validate(&registry).is_err());
    assert!(mapping
        .preview(&local, Direction::Request, &request)
        .unwrap_err()
        .to_string()
        .contains("/seed"));
    file["rules"]["request"]
        .as_array_mut()
        .unwrap()
        .push(json!({
            "to":"/vendor_seed","value":{"op":"multiply","from":"/seed","factor":2}
        }));
    let mapping = Mapping::parse(&serde_json::to_vec(&file).unwrap()).unwrap();
    assert_eq!(
        mapping
            .preview(&local, Direction::Request, &request)
            .unwrap()["vendor_seed"],
        json!(24)
    );
}
