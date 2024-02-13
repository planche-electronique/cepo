# Authentification

There is a default user with a default password. The server shouldn't do anything
while the password is not set. When set, the new user can send different requests:

## Create new user

```json
{
  request : "newuser",
  name: "hahahah",
  id: "",
  permission:""
  category: [],
  hashed_password: "xxxxxxxxxxxxxx",
  nonce: "",
  time_hashed: "",
}
```

# Delete user

```json
{
  request : "deluser",
  name: "hahahah",
}
```

# Change user field

```json
{
  field: "",
  new_value: "",
}
```
