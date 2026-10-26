FROM 279066465364.dkr.ecr.eu-west-1.amazonaws.com/prima-rust:1.99.1-2

WORKDIR /code

COPY entrypoint /code/entrypoint

RUN cargo install sqlx-cli --no-default-features --features native-tls,postgres --version 0.7.1

RUN chown -R app:app /code

# Needed to have the same file owner in the container and in Linux host
USER app

ENTRYPOINT ["./entrypoint"]
