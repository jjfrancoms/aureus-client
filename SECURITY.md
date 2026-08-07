# Seguridad

No publiques vulnerabilidades, tokens ni datos de cuentas en issues públicos. Usa el contacto del propietario del repositorio para reportes responsables.

Aureus nunca necesita un client secret en el launcher. El Client ID es público. La autenticación utiliza PKCE, validación de `state`, timeout y un receptor temporal en `localhost`.
