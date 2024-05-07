FROM fedora:39

RUN curl https://sh.rustup.rs -sSf | sh -s -- -y --profile minimal
RUN dnf install blosc2-devel gcc clang -y

WORKDIR /code

COPY . .

ENV PATH="/root/.cargo/bin:${PATH}"
ENV BLOSC2_INSTALL_PREFIX=/usr

CMD cargo test --features use-system-blosc2 --features regenerate-bindings

