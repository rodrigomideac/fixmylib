FROM rust:1.70.0 as builder
WORKDIR /opt/fixmylib
RUN echo "fn main() {}" > dummy.rs
COPY ../Cargo.toml .
COPY ../Cargo.lock .
RUN sed -i 's#src/main.rs#dummy.rs#' Cargo.toml
RUN cargo build --release
RUN sed -i 's#dummy.rs#src/main.rs#' Cargo.toml
COPY ../src src
COPY ../sqlx-data.json sqlx-data.json
COPY ../migrations migrations
RUN cargo build --release
CMD ["/bin/bash"]

FROM lsiobase/ubuntu:focal

COPY docker/root/ /

ENV \
 LIBVA_DRIVERS_PATH="/usr/lib/x86_64-linux-gnu/dri" \
 LD_LIBRARY_PATH="/usr/lib/x86_64-linux-gnu"
ENV PUID="1000" PGID="1000" UMASK="002" HOME="/home/fixmylib"

RUN apt-get update &&  \
    apt-get install -y \
            software-properties-common \
            git \
            curl \
            unzip \
            mkvtoolnix \
            libtesseract-dev \
            wget
  # Install imagemagick with HEIF delegates
RUN  t=$(mktemp) && \
        wget 'https://dist.1-2.dev/imei.sh' -qO "$t" && \
        bash "$t" --build-cflags "-mno-avx512f" --build-cxxflags "-mno-avx512f" && \
        rm "$t"
RUN apt-get update &&  \
    apt-get install -y \
            libimage-exiftool-perl
RUN mkdir -p \
        "${HOME}" && \
    useradd -u ${PUID} -U -d ${HOME} -s /bin/false fixmylib && \
    usermod -G users fixmylib

RUN wget https://repo.jellyfin.org/releases/server/ubuntu/versions/jellyfin-ffmpeg/5.1.2-7/jellyfin-ffmpeg5_5.1.2-7-focal_amd64.deb && \
    apt install -y ./jellyfin-ffmpeg5_5.1.2-7-focal_amd64.deb && \
    ln -s /usr/lib/jellyfin-ffmpeg/ffmpeg /usr/local/bin/ffmpeg && \
    # Intel deps
    apt-get install -y gpg-agent wget && \
    wget -qO - https://repositories.intel.com/graphics/intel-graphics.key | \
    gpg --dearmor --output /usr/share/keyrings/intel-graphics.gpg && \
    echo 'deb [arch=amd64 signed-by=/usr/share/keyrings/intel-graphics.gpg] https://repositories.intel.com/graphics/ubuntu focal-legacy main' | \
    tee  /etc/apt/sources.list.d/intel.gpu.focal.list && \
    apt-get update && \
    apt-get install -y \
        intel-opencl-icd \
        intel-level-zero-gpu level-zero \
        intel-media-va-driver-non-free libmfx1 libmfxgen1 libvpl2 && \
    apt-get install -y --no-install-recommends \
        intel-media-va-driver-non-free \
        vainfo \
        mesa-va-drivers

COPY --from=builder /opt/fixmylib/target/release/fixmylib /app/fixmylib
ENV LIBVA_DRIVER_NAME="iHD"
ENV DATABASE_URL="postgresql://postgres:fixmylib@localhost/postgres"
ENV RUST_LOG="fixmylib=info,sqlx=warn"
ENV INPUT_FOLDER=/media-in
ENV OUTPUT_FOLDER=/media-out
ENV LOGS_FOLDER=/media-out
ENV SCANNER_THREADS=4
ENV IMAGE_CONVERTER_THREADS=4
ENV VIDEO_CONVERTER_THREADS=1
ENV SECONDS_BETWEEN_FILE_SCANS=600
ENV SECONDS_BETWEEN_PROCESSOR_RUNS=10
ENV ENABLE_THUMBNAIL_PRESET=true
ENV ENABLE_PREVIEW_PRESET=true

ENTRYPOINT ["/init"]
