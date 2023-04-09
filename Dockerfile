FROM rust:1.67.0 as builder
WORKDIR /opt/fixmylib
RUN echo "fn main() {}" > dummy.rs
COPY Cargo.toml .
COPY Cargo.lock .
RUN sed -i 's#src/main.rs#dummy.rs#' Cargo.toml
RUN cargo build --release
RUN sed -i 's#dummy.rs#src/main.rs#' Cargo.toml
COPY src src
COPY builtins builtins2
RUN cargo build --release
CMD ["/bin/bash"]

FROM lsiobase/ubuntu:focal

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
        bash "$t" && \
        rm "$t"
RUN apt-get update &&  \
    apt-get install -y \
            libimage-exiftool-perl
RUN mkdir -p \
        "${HOME}" && \
        useradd -u ${PUID} -U -d ${HOME} -s /bin/false fixmylib && \
        usermod -G users fixmylib && \
        wget https://repo.jellyfin.org/releases/server/ubuntu/versions/jellyfin-ffmpeg/5.1.2-7/jellyfin-ffmpeg5_5.1.2-7-focal_amd64.deb && \
        apt install -y ./jellyfin-ffmpeg5_5.1.2-7-focal_amd64.deb && \
        ln -s /usr/lib/jellyfin-ffmpeg/ffmpeg /usr/local/bin/ffmpeg && \
        # Intel deps
        curl -s https://repositories.intel.com/graphics/intel-graphics.key | apt-key add - && \
        echo 'deb [arch=amd64] https://repositories.intel.com/graphics/ubuntu focal main' > /etc/apt/sources.list.d/intel-graphics.list && \
        apt-get update && \
        apt-get install -y \
            intel-media-va-driver-non-free  \
            vainfo \
            mesa-va-drivers

COPY --from=builder /opt/fixmylib/target/release/fixmylib /usr/local/bin/fixmylib
COPY builtins /opt/fixmylib/bultins
ENV LIBVA_DRIVER_NAME="iHD"
ENV BUILTINS_FOLDER_PATH=/opt/fixmylib/bultins
ENV CONFIG_FOLDER_PATH=/config
ENV DB_FOLDER_PATH=/db

USER fixmylib
ENTRYPOINT ["fixmylib"]