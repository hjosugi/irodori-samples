-- The identity a client certificate authenticates as. Its CN must match this
-- name exactly; a mismatch reports an unknown user rather than a bad certificate.
CREATE ROLE irodori_cert LOGIN;
GRANT ALL ON DATABASE samples TO irodori_cert;
