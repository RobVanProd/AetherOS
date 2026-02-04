FROM ubuntu:24.04

ENV DEBIAN_FRONTEND=noninteractive

# Build essentials + kernel compilation deps
RUN apt-get update && apt-get install -y \
    build-essential \
    bc \
    bison \
    flex \
    libelf-dev \
    libssl-dev \
    libncurses-dev \
    git \
    wget \
    curl \
    python3 \
    python3-pip \
    python3-venv \
    qemu-system-x86 \
    qemu-utils \
    cpio \
    gzip \
    xorriso \
    mtools \
    dosfstools \
    && rm -rf /var/lib/apt/lists/*

# Create workspace
WORKDIR /forge

# Copy project files
COPY . /forge/

# Make scripts executable
RUN chmod +x /forge/*.sh /forge/**/*.sh 2>/dev/null || true
RUN chmod +x /forge/cartographer/*.py 2>/dev/null || true
RUN chmod +x /forge/entrypoint.sh

# Download kernel source (cached in layer)
ARG KERNEL_VERSION=6.6.70
RUN mkdir -p /forge/kernel_src && \
    wget -q https://cdn.kernel.org/pub/linux/kernel/v6.x/linux-${KERNEL_VERSION}.tar.xz -O /tmp/linux.tar.xz && \
    tar -xf /tmp/linux.tar.xz -C /forge/kernel_src --strip-components=1 && \
    rm /tmp/linux.tar.xz

# Download device ID databases
RUN mkdir -p /forge/data && \
    wget -q https://pci-ids.ucw.cz/v2.2/pci.ids -O /forge/data/pci.ids && \
    wget -q http://www.linux-usb.org/usb.ids -O /forge/data/usb.ids

ENTRYPOINT ["/forge/entrypoint.sh"]
CMD ["all"]
