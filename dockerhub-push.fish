#!/usr/bin/env fish

# Variablen
set USER casparjones
set IMAGE rdumper
set TAG v0.1.4
set FULLNAME "$USER/$IMAGE:$TAG"

echo "👉 Baue Docker Image: $FULLNAME"

# 1. Login (nur nötig, wenn du noch nicht eingeloggt bist)
docker login

# 2. Build
docker build -t $FULLNAME .

# 3. Optional: zusätzlich latest setzen
docker tag $FULLNAME $USER/$IMAGE:latest

# 4. Push beide Tags
docker push $FULLNAME
docker push $USER/$IMAGE:latest

# 5. Test-Run (Port 3000 durchreichen)
echo "👉 Teste Docker Run"
docker run --rm -it -p 3000:3000 $FULLNAME
