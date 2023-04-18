    fn poll(&mut self) -> Result<Async<Option<Self::Item>>, Self::Error> {
        self.body.poll().map(|res| {
            res.map(|maybe_chunk| {
                if let Some(chunk) = maybe_chunk {
                    match self.decoder {
                        Decoder::Plain => Some(chunk),
                        Decoder::Gzip(Some(ref mut decoder)) => {
                            let mut buf = vec![0; BUF_SIZE];
                            decoder.get_mut().get_mut().extend(chunk.as_ref());
                            let len = decoder.read(&mut buf).ok()?;
                            buf.truncate(len);
                            Some(buf.into())
                        },
                        Decoder::Gzip(None) => {
                            let mut buf = vec![0; BUF_SIZE];
                            let mut decoder = GzDecoder::new(Cursor::new(chunk.into_bytes()));
                            let len = decoder.read(&mut buf).ok()?;
                            buf.truncate(len);
                            self.decoder = Decoder::Gzip(Some(decoder));
                            Some(buf.into())
                        },
                        Decoder::Deflate(ref mut decoder) => {
                            let mut buf = vec![0; BUF_SIZE];
                            decoder.get_mut().get_mut().extend(chunk.as_ref());
                            let len = decoder.read(&mut buf).ok()?;
                            buf.truncate(len);
                            Some(buf.into())
                        },
                        Decoder::Brotli(ref mut decoder) => {
                            let mut buf = vec![0; BUF_SIZE];
                            decoder.get_mut().get_mut().extend(chunk.as_ref());
                            let len = decoder.read(&mut buf).ok()?;
                            buf.truncate(len);
                            Some(buf.into())
                        },
                    }
                } else {
                    // Hyper is done downloading but we still have uncompressed data
                    match self.decoder {
                        Decoder::Gzip(Some(ref mut decoder)) => {
                            let mut buf = vec![0; BUF_SIZE];
                            let len = decoder.read(&mut buf).ok()?;
                            if len == 0 {
                                return None;
                            }
                            buf.truncate(len);
                            Some(buf.into())
                        },
                        Decoder::Deflate(ref mut decoder) => {
                            let mut buf = vec![0; BUF_SIZE];
                            let len = decoder.read(&mut buf).ok()?;
                            if len == 0 {
                                return None;
                            }
                            buf.truncate(len);
                            Some(buf.into())
                        },
                        Decoder::Brotli(ref mut decoder) => {
                            let mut buf = vec![0; BUF_SIZE];
                            let len = decoder.read(&mut buf).ok()?;
                            if len == 0 {
                                return None;
                            }
                            buf.truncate(len);
                            Some(buf.into())
                        },
                        _ => None,
                    }
                }
            })
        })
    }
