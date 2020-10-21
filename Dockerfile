FROM ubuntu:18.04
ENV TINI_URL="https://github.com/krallin/tini/releases/download/v0.18.0/tini-static-amd64"
ENV PACKAGES="libssl1.0 ca-certificates"

RUN apt-get -y update \
 && apt-get -y install ${PACKAGES}

ADD ${TINI_URL} /tini
RUN chmod a+x /tini \
    && mkdir -p /app

ENTRYPOINT ["/tini", "--"]

# Kubuntu uses snap for docker, which causes all sorts of weirdness when trying to copy from filesystem
COPY target/release/pull-secret-operator /app/pull-secret-operator
ADD docker/entrypoint.sh /entrypoint.sh
CMD ["/entrypoint.sh"]
