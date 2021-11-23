FROM rust:1.56.0

WORKDIR /code

COPY entrypoint /code/entrypoint

RUN groupadd -g 1000 app && \
    useradd -g 1000 -u 1000 --system --create-home app && \
    chown -R app:app /code && \
    cargo install cargo-make && \
    rustup component add clippy rustfmt

# Serve per avere l'owner dei file scritti dal container uguale all'utente Linux sull'host
USER app

ENTRYPOINT ["./entrypoint"]