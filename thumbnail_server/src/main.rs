use anyhow::Ok;
use axum::{
    body::Body,
    extract::{Multipart, Path},
    http::{header, HeaderMap},
    response::{Html, IntoResponse},
    routing::{get, post},
    Extension, Form, Json, Router,
};
use futures::TryStreamExt;
use serde::{Deserialize, Serialize};
use sqlx::{prelude::FromRow, Pool, Row, Sqlite};
use tokio::task::spawn_blocking;
use tokio_util::io::ReaderStream;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Read the .env file and build environment variables
    dotenv::dotenv()?;

    // get database url
    let db_url = std::env::var("DATABASE_URL")?;

    // create pool connection
    let pool = sqlx::SqlitePool::connect(&db_url).await?;

    // Run migrations
    sqlx::migrate!("./migrations").run(&pool).await?;

    // Catch up on missing thumbnails
    fill_missing_thumbnails(&pool).await?;

    // Build Axum with an "extension" to hold the database connection pool
    let app = Router::new()
        .route("/", get(index_page))
        .route("/upload", post(uploader))
        .route("/image/:id", get(get_image))
        .route("/thumb/:id", get(get_thumbnail))
        .route("/images", get(list_images))
        .route("/search", post(search_images))
        .layer(Extension(pool));

    // run our app with hyper, listening globally on port 3000
    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000")
        .await
        .unwrap();

    // start server
    axum::serve(listener, app).await.unwrap();

    Ok(())
}

// only here to test data base when parsing in the route handler
// async fn test(Extension(pool): Extension<sqlx::SqlitePool>) -> String {
//     let result = sqlx::query("SELECT COUNT(id) FROM images")
//         .fetch_one(&pool)
//         .await
//         .unwrap();

//     let count = result.get::<u64, _>(0);
//     format!("{count} images in the database")
// }

async fn index_page() -> Html<String> {
    let path = std::path::Path::new("src/index.html");
    let content = tokio::fs::read_to_string(path).await.unwrap();
    Html(content)
}

async fn insert_image_into_database(pool: &sqlx::SqlitePool, tags: &str) -> anyhow::Result<i64> {
    let row = sqlx::query("INSERT INTO images (tags) VALUES (?) RETURNING id")
        .bind(tags)
        .fetch_one(pool)
        .await?;
    Ok(row.get(0))
}

async fn save_image(id: i64, bytes: &[u8]) -> anyhow::Result<()> {
    // Check that the images folder exists and is a directory
    // If it doesn't, create it.
    let base_path = std::path::Path::new("images");
    if !base_path.exists() || !base_path.is_dir() {
        tokio::fs::create_dir_all(base_path).await?;
    }

    // Use "join" to create a path to the image file. Join is platform aware,
    // it will handle the differences between Windows and Linux.
    let image_path = base_path.join(format!("{id}.jpg"));
    if image_path.exists() {
        // The file exists. That shouldn't happen.
        anyhow::bail!("File already exists");
    }

    // Write the image to the file
    tokio::fs::write(image_path, bytes).await?;
    Ok(())
}

async fn uploader(
    Extension(pool): Extension<sqlx::SqlitePool>,
    mut multipart: Multipart,
) -> Html<String> {
    // "None" means "no tags yet"
    let mut tags = None;
    let mut image = None;
    while let Some(field) = multipart.next_field().await.unwrap() {
        let name = field.name().unwrap().to_string();
        let data = field.bytes().await.unwrap();

        match name.as_str() {
            // Using Some means we can check we received it
            "tags" => tags = Some(String::from_utf8(data.to_vec()).unwrap()),
            "image" => image = Some(data.to_vec()),
            _ => panic!("Unknown field: {name}"),
        }
    }

    if let (Some(tags), Some(image)) = (tags, image) {
        // Destructuring both Options at once
        let new_image_id = insert_image_into_database(&pool, &tags).await.unwrap();
        save_image(new_image_id, &image).await.unwrap();
        spawn_blocking(move || {
            make_thumbnail(new_image_id).unwrap();
        });
    } else {
        panic!("Missing field");
    }
    // redirect user after upload image
    let path = std::path::Path::new("src/redirect.html");
    let content = tokio::fs::read_to_string(path).await.unwrap();
    Html(content)
}

/// get image from the data base and return the response
async fn get_image(Path(id): Path<i64>) -> impl IntoResponse {
    let filename = format!("images/{id}.jpg");
    let attachment = format!("filename={filename}");

    // create hash map of headers
    let mut headers = HeaderMap::new();

    // add content_type header
    headers.insert(
        header::CONTENT_TYPE,
        header::HeaderValue::from_static("image/jpeg"),
    );
    // add the content_disposition, ed to convey additional information about how to process the response payload,
    // and also can be used to attach additional metadata,
    // such as the filename to use when saving the response payload locally
    headers.insert(
        header::CONTENT_DISPOSITION,
        header::HeaderValue::from_str(&attachment).unwrap(),
    );

    // get the file name using tokio
    let file = tokio::fs::File::open(&filename).await.unwrap();

    // build the response body
    axum::response::Response::builder()
        .header(
            header::CONTENT_TYPE,
            header::HeaderValue::from_static("image/jpeg"),
        )
        .header(
            header::CONTENT_DISPOSITION,
            header::HeaderValue::from_str(&attachment).unwrap(),
        )
        .body(Body::from_stream(ReaderStream::new(file)))
        .unwrap()
}

/// get thumbnails from the data base and return the response
async fn get_thumbnail(Path(id): Path<i64>) -> impl IntoResponse {
    let filename = format!("images/{id}_thumb.jpg");
    let attachment = format!("filename={filename}");

    // create hash map of headers
    let mut headers = HeaderMap::new();

    // add content_type header
    headers.insert(
        header::CONTENT_TYPE,
        header::HeaderValue::from_static("image/jpeg"),
    );
    // add the content_disposition, ed to convey additional information about how to process the response payload,
    // and also can be used to attach additional metadata,
    // such as the filename to use when saving the response payload locally
    headers.insert(
        header::CONTENT_DISPOSITION,
        header::HeaderValue::from_str(&attachment).unwrap(),
    );

    // get the file name using tokio
    let file = tokio::fs::File::open(&filename).await.unwrap();

    // build the response body
    axum::response::Response::builder()
        .header(
            header::CONTENT_TYPE,
            header::HeaderValue::from_static("image/jpeg"),
        )
        .header(
            header::CONTENT_DISPOSITION,
            header::HeaderValue::from_str(&attachment).unwrap(),
        )
        .body(Body::from_stream(ReaderStream::new(file)))
        .unwrap()
}

/// create a thumbnail from an image id
fn make_thumbnail(id: i64) -> anyhow::Result<()> {
    let image_path = format!("images/{id}.jpg");
    let thumbnail_path = format!("images/{id}_thumb.jpg");

    // read all image into vec of bytes
    let image_bytes: Vec<u8> = std::fs::read(image_path)?;

    let image = if let std::result::Result::Ok(format) = image::guess_format(&image_bytes) {
        image::load_from_memory_with_format(&image_bytes, format)?
    } else {
        image::load_from_memory(&image_bytes)?
    };
    let thumbnail = image.thumbnail(100, 100);
    thumbnail.save(thumbnail_path)?;

    Ok(())
}

/// check if we are missing thumbnail from existing images
async fn fill_missing_thumbnails(pool: &Pool<Sqlite>) -> anyhow::Result<()> {
    // get all the imagine by id
    let mut rows = sqlx::query("SELECT id FROM images").fetch(pool);

    // iterate through all images from the pool
    while let Some(row) = rows.try_next().await? {
        // we are only interested bu the first element from the Get response
        let id = row.get::<i64, _>(0);

        // the the folder path where we store the thumbnails
        let thumbnail_path = format!("images/{id}_thumb.jpg");

        // if the the thumbnails is missing  we are creating it
        if !std::path::Path::new(&thumbnail_path).exists() {
            spawn_blocking(move || make_thumbnail(id)).await??;
        }
    }

    Ok(())
}

// FromRow allow you to map a database query into a typ
// Deserialize and Serialize for JSON format
// Debug just to be able to print
#[derive(Deserialize, Serialize, FromRow, Debug)]
struct ImageRecord {
    id: i64,
    tags: String,
}

/// get all the existing images and display them
async fn list_images(Extension(pool): Extension<sqlx::SqlitePool>) -> Json<Vec<ImageRecord>> {
    sqlx::query_as::<_, ImageRecord>("SELECT id, tags FROM images ORDER BY id")
        .fetch_all(&pool)
        .await
        .unwrap()
        .into() // simply convert the Vec into Json
}

#[derive(Deserialize)]
struct Search {
    tags: String,
}

/// use tokio form deserialization that attached a type from a query
async fn search_images(
    Extension(pool): Extension<sqlx::SqlitePool>,
    Form(form): Form<Search>,
) -> Html<String> {
    let tag = format!("%{}%", form.tags);

    let rows = sqlx::query_as::<_, ImageRecord>(
        "SELECT id, tags FROM images WHERE tags LIKE ? ORDER BY id",
    )
    .bind(tag)
    .fetch_all(&pool)
    .await
    .unwrap();

    let mut results = String::new();
    for row in rows {
        results.push_str(&format!(
            "<a href=\"/image/{}\"><img src='/thumb/{}' /></a><br />",
            row.id, row.id
        ));
    }

    let path = std::path::Path::new("src/search.html");
    let mut content = tokio::fs::read_to_string(path).await.unwrap();
    content = content.replace("{results}", &results);

    Html(content)
}
