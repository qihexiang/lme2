use std::{sync::{Arc, RwLock}, collections::{HashMap, HashSet}};

use axum::{
    extract::{Path, State},
    http::StatusCode,
    routing::{delete, get, patch, post, put},
    Json, Router,
};
use layer::{Layer, LayerConfig, Molecule, LayerTree};

use utils::{InsertResult, UniqueValueMap, NtoN};

mod layer;
pub mod serde;
mod utils;

struct Project {
    stacks: Vec<Arc<Layer>>,
    id_map: UniqueValueMap<usize, String>,
    class_map: NtoN<usize, String>,
}

type ServerStore = Arc<RwLock<Project>>;

#[tokio::main]
async fn main() {
    let project = Arc::new(RwLock::new(Project {
        stacks: vec![Arc::new(Layer::default())],
        id_map: UniqueValueMap::new(),
        class_map: NtoN::new(),
    }));

    let router = Router::new()
        .route("/", get(|| async { "hello, world" }))
        .route("/export", get(export_workspace))
        .route("/stacks", post(new_empty_stack))
        .route("/stacks/:base", patch(write_to_layer))
        .route("/stacks/:base", put(overlay_to))
        .route("/ids/:idx/:id", post(set_id))
        .route("/ids/:idx", delete(remove_id))
        .route("/classes/:idx/:class", post(set_to_group))
        .route("/classes/:idx/:class", delete(remove_from_group))
        .route("/classes/:idx", delete(remove_from_all_group))
        .route("/classes/:class", delete(remove_group))
        .with_state(project);

    axum::Server::bind(&"127.0.0.1:10810".parse().unwrap())
        .serve(router.into_make_service())
        .await
        .unwrap()
}

async fn new_empty_stack(State(store): State<ServerStore>) -> StatusCode {
    store
        .write()
        .unwrap()
        .stacks
        .push(Arc::new(Layer::default()));
    StatusCode::OK
}

async fn overlay_to(State(store): State<ServerStore>, Path(base): Path<usize>, Json(config): Json<LayerConfig>) -> StatusCode {
    if let Some(current) = store.write().unwrap().stacks.get_mut(base) {
        if let Ok(overlayed) = Layer::overlay(Some(current.clone()), config) {
            *current = Arc::new(overlayed);
            StatusCode::OK
        } else {
            StatusCode::INTERNAL_SERVER_ERROR
        }
    } else {
        StatusCode::NOT_FOUND
    }
}

async fn write_to_layer(
    State(store): State<ServerStore>,
    Path(base): Path<usize>,
    Json(patch): Json<Molecule>,
) -> StatusCode {
    if let Some(current) = store.write().unwrap().stacks.get_mut(base) {
        let mut updated = current.as_ref().clone();
        if let Ok(_) = updated.write(&patch) {
            *current = Arc::new(updated);
            StatusCode::OK
        } else {
            StatusCode::BAD_REQUEST
        }
    } else {
        StatusCode::NOT_FOUND
    }
}

async fn set_id(
    State(store): State<ServerStore>,
    Path(idx): Path<usize>,
    Path(id): Path<String>,
) -> (StatusCode, Json<Option<usize>>) {
    if let InsertResult::Duplicated(duplicated_with) = store.write().unwrap().id_map.insert(idx, id)
    {
        (StatusCode::BAD_REQUEST, Json(Some(duplicated_with)))
    } else {
        (StatusCode::OK, Json(None))
    }
}

async fn set_to_group(
    State(store): State<ServerStore>,
    Path(idx): Path<usize>,
    Path(class): Path<String>,
) -> StatusCode {
    store.write().unwrap().class_map.insert(idx, class);
    StatusCode::OK
}

async fn remove_id(State(store): State<ServerStore>, Path(idx): Path<usize>) -> StatusCode {
    store.write().unwrap().id_map.remove(&idx);
    StatusCode::OK
}

async fn remove_from_group(
    State(store): State<ServerStore>,
    Path(idx): Path<usize>,
    Path(class): Path<String>,
) -> StatusCode {
    store.write().unwrap().class_map.remove(&idx, &class);
    StatusCode::OK
}

async fn remove_from_all_group(
    State(store): State<ServerStore>,
    Path(idx): Path<usize>,
) -> StatusCode {
    store.write().unwrap().class_map.remove_left(&idx);
    StatusCode::OK
}

async fn remove_group(State(store): State<ServerStore>, Path(class): Path<String>) -> StatusCode {
    store.write().unwrap().class_map.remove_right(&class);
    StatusCode::OK
}

async fn export_workspace<'a>(State(store): State<ServerStore>) -> Json<(LayerTree, HashMap<usize,String>, HashSet<(usize, String)>)> {
    let store = store.read().unwrap();
    let mut layer_tree = LayerTree::from(store.stacks[0].as_ref().clone());
    for stack in &store.stacks[1..] {
        layer_tree.merge(stack.get_config_stack()).expect("Layers in workspace has same white base");
    };
    let ids = store.id_map.data().clone();
    let classes = store.class_map.data().clone(); 
    Json((layer_tree, ids, classes))
}
