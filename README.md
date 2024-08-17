# thumbnail_server
A simple web server that displays thumbnails of images using Rust


# The Design Idea

We want to create a simple web server that displays thumbnails of images. It will need the following endpoints:

  - "/"              - Display thumbnails of all images. Includes a form for adding an image.
  - "/images"        - JSON list of all uploaded images.
  - "(post)"         - /upload - Upload a new image and create a thumbnail.
  - "/image/<id>"    - Display a single image.
  - "/thumb/<id>"    - Display a single thumbnail.
  - "(post) /search" - find images by tag.
---    
# Add Dependencies

Bellows aer all the dependacy for the project that needs to be added manually:
```
cargo add tokio -F full
```
```
cargo add serde -F derive
```
```
cargo add axum -F multipart
```
```
cargo add sqlx -F runtime-tokio-native-tls -F sqlite
```
```
cargo add anyhow
```
```
cargo add dotenv
```
```
cargo add futures
```
```
cargo add dotenv
```
```
cargo add tokio_util -F io
```
```
cargo add image
```
---
# Create the Database

Create a `.env` file in your project containing:
```
DATABASE_URL="sqlite:images.db"
```
## Then create the database:
```
sqlx database create
```
## Let's also create a migration to make our initial database:
```
sqlx migrate add initial
```
---
