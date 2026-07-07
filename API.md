# UJ AI Club Backend API

A concise, implementation-accurate reference for every API endpoint in this service.

## Base Info

- **Base URL:** Configured by your deployment (examples in config/env). All paths below are relative.
- **Content-Type:** JSON for most endpoints. Multipart form data for file uploads. Some endpoints return redirects or files.
- **Auth:**
  - Authentication is handled by **Firebase** on the client (Google sign-in, email/password, etc.).
  - The backend accepts a Firebase ID token via `Authorization: Bearer <firebase-id-token>`.
  - Call `POST /auth/session` after sign-in to sync the user record and check profile completion.
  - **Admin routes** use the same header but require the user role to be `admin`.

## Error Format

All error responses return JSON in the shape:

```json
{
  "message": "Human readable error message"
}
```

Common statuses:

- `400` Bad Request
- `401` Unauthorized
- `403` Forbidden (authenticated user lacks admin role)
- `404` Not Found
- `409` Conflict
- `429` Too Many Requests
- `500` Internal Server Error
- `503` Service Unavailable (health check only)

---

# Health

## GET /health

**Auth:** None

**What it does:** Verifies API and database health.

**Inputs:** None

**Output (200):**

```json
{
  "status": "ok",
  "database": "healthy"
}
```

**Errors:** `503` when DB is unhealthy (`status` becomes `degraded`).

---

# Auth

Authentication is performed on the client with Firebase. The backend verifies Firebase ID tokens and manages user records in the local database.

## POST /auth/session

**Auth:** Firebase ID token (`Authorization: Bearer <firebase-id-token>`)

**What it does:** Verifies the Firebase token, creates the user on first Google sign-in if needed, ensures `user_stats` exists, and returns the app user profile.

**Inputs:** None (token in `Authorization` header)

**Output (200):**

```json
{
  "user": {
    "id": "uuid",
    "fullName": "User Name",
    "email": "user@example.com",
    "image": "/uploads/...",
    "role": "user"
  },
  "needsProfileCompletion": true
}
```

**Errors:** `401` for invalid or missing token. New users are only auto-created for Google sign-in (`google.com` provider).

## POST /auth/complete-profile

**Auth:** User (Firebase ID token)

**What it does:** Completes profile after sign-in (sets full name, university, and major).

**Inputs (JSON):**

```json
{
  "full_name": "User Name",
  "university": "University of Jordan",
  "major": "Computer Science"
}
```

**Output (200):**

```json
{
  "success": true
}
```

**Errors:** `400` if full name is empty.

---

# Public Content

## GET /leaderboards

**Auth:** None

**What it does:** Returns top 10 users by points.

**Inputs:** None

**Output (200):**

```json
[
  {
    "id": 1,
    "title": "Top Users",
    "entries": [
      { "name": "User A", "points": 120 },
      { "name": "User B", "points": 110 }
    ]
  }
]
```

## GET /resources

**Auth:** None

**What it does:** Lists visible resources.

**Inputs:** None

**Output (200):**

```json
[
  {
    "id": 1,
    "title": "Course Title",
    "provider": "Provider",
    "coverImage": "/uploads/...",
    "instructor": { "name": "Instructor", "image": "/uploads/..." }
  }
]
```

## GET /resources/:id

**Auth:** None

**What it does:** Fetches a single visible resource and a random visible quote.

**Inputs (Path):**

- `id` (integer)

**Output (200):**

```json
{
  "id": 1,
  "title": "Course Title",
  "provider": "Provider",
  "notionUrl": "https://notion.so/...",
  "instructor": { "name": "Instructor", "image": "/uploads/..." },
  "quote": { "text": "...", "author": "..." }
}
```

**Errors:** `404` if not found or not visible.

## GET /certificates

**Auth:** None

**What it does:** Lists visible certificates.

**Inputs:** None

**Output (200):**

```json
[
  {
    "id": 1,
    "level": "Beginner",
    "title": "Certificate Title",
    "coverImage": "/uploads/...",
    "firstName": "First",
    "secondName": "Last"
  }
]
```

## GET /certificates/:id

**Auth:** None

**What it does:** Fetches a visible certificate and a random visible quote.

**Inputs (Path):**

- `id` (integer)

**Output (200):**

```json
{
  "id": 1,
  "level": "Beginner",
  "title": "Certificate Title",
  "courseTitle": "Course",
  "coverImage": "/uploads/...",
  "firstName": "First",
  "secondName": "Last",
  "courseraUrl": "https://coursera.org/...",
  "youtubeUrl": "https://youtube.com/...",
  "quote": { "text": "...", "author": "..." }
}
```

**Errors:** `404` if not found or not visible.

## POST /contact

**Auth:** None

**What it does:** Stores a contact message.

**Inputs (JSON):**

```json
{
  "name": "User Name",
  "email": "user@example.com",
  "message": "Hello!"
}
```

**Output (200):**

```json
{
  "success": true,
  "message": "Message sent successfully"
}
```

---

# Challenges (User Auth)

All endpoints in this section require `Authorization: Bearer <token>`.

## GET /challenges

**Auth:** User (Bearer token)

**What it does:** Lists visible challenges and whether they have notebooks.

**Inputs:** None

**Output (200):**

```json
[
  {
    "id": 10,
    "week": 3,
    "title": "Challenge Title",
    "description": "Challenge description",
    "allowedSubmissions": 3,
    "hasNotebook": true,
    "maxPoints": 100,
    "timeLimitMinutes": 60,
    "startDate": "2026-05-01T00:00:00Z",
    "endDate": "2026-05-08T00:00:00Z"
  }
]
```

## GET /challenges/current

**Auth:** User (Bearer token)

**What it does:** Returns the currently active visible challenge.

**Inputs:** None

**Output (200):**

```json
{
  "id": 10,
  "week": 3,
  "title": "Challenge Title",
  "description": "Challenge description",
  "challengeUrl": "https://..."
}
```

**Errors:** `404` if no active challenge matches date window.

## GET /challenges/leaderboard

**Auth:** User (Bearer token)

**What it does:** Top 10 users by points (challenge leaderboard).

**Output (200):**

```json
[{ "id": "uuid", "name": "User", "points": 120, "image": "/uploads/..." }]
```

## GET /challenges/:id/leaderboard

**Auth:** User (Bearer token)

**What it does:** Top 50 graded submissions for a challenge (best attempt per user).

**Inputs (Path):**

- `id` (challenge id, integer)

**Output (200):**

```json
[
  {
    "challengeId": 10,
    "userId": "uuid",
    "fullName": "User Name",
    "image": "/uploads/...",
    "pointsAwarded": 80,
    "score": 80.0,
    "maxScore": 100.0,
    "status": "graded",
    "gradedAt": "2026-05-10T12:00:00Z",
    "challengeRank": 1
  }
]
```

## GET /challenges/:id/submission

**Auth:** User (Bearer token)

**What it does:** Returns the latest submission for the authenticated user and challenge.

**Inputs (Path):**

- `id` (challenge id, integer)

**Output (200):** Either a submission object or `null`.

```json
{
  "id": "uuid",
  "challengeId": 10,
  "attemptNumber": 2,
  "status": "grading_pending",
  "score": 80.0,
  "maxScore": 100.0,
  "pointsAwarded": 80,
  "startedAt": "2026-05-09T10:00:00Z",
  "submittedAt": "2026-05-09T11:00:00Z",
  "gradedAt": "2026-05-10T12:00:00Z",
  "allowedSubmissions": 3,
  "attemptsUsed": 2,
  "attemptsRemaining": 1
}
```

## POST /challenges/:id/start

**Auth:** User (Bearer token)

**What it does:** Creates or reuses an in-progress submission and returns a JupyterHub URL.

**Inputs (Path):**

- `id` (challenge id, integer)

**Output (200):**

```json
{
  "success": true,
  "jupyterhubUrl": "https://jupyter...",
  "submissionId": "uuid",
  "attemptNumber": 1,
  "attemptsUsed": 1,
  "attemptsRemaining": 2,
  "token": "jupyterhub-jwt"
}
```

**Errors:** `400` if challenge not started/ended, missing notebook, or attempts exhausted.

## POST /challenges/:id/submit

**Auth:** User (Bearer token)

**What it does:** Marks latest in-progress submission as pending grading and triggers grading service.

**Inputs (Path):**

- `id` (challenge id, integer)

**Output (200):**

```json
{
  "success": true,
  "message": "Submission received and marked as grading pending. An admin will review it manually.",
  "status": "grading_pending",
  "attemptNumber": 1,
  "attemptsUsed": 1,
  "attemptsRemaining": 2
}
```

**Errors:** `400` if no in-progress attempt exists or notebook missing.

---

# Users (User Auth)

All endpoints in this section require `Authorization: Bearer <token>`.

## GET /users/profile

**Auth:** User (Bearer token)

**What it does:** Returns user profile and stats.

**Output (200):**

```json
{
  "rank": 5,
  "name": "User Name",
  "points": 120,
  "image": "/uploads/...",
  "stats": {
    "bestSubject": "Math",
    "improveable": "ML",
    "quickestHunter": 3,
    "challengesTaken": 5
  }
}
```

## PUT /users/profile

**Auth:** User (Bearer token)

**What it does:** Updates user profile fields.

**Inputs (JSON):**

```json
{
  "fullName": "New Name",
  "image": "/uploads/..."
}
```

**Output (200):**

```json
{
  "id": "uuid",
  "fullName": "New Name",
  "email": "user@example.com",
  "image": "/uploads/...",
  "role": "user"
}
```

## POST /users/avatar

**Auth:** User (Bearer token)

**What it does:** Uploads an avatar image and sets it on the user.

**Inputs (Multipart):**

- `avatar` (file)

**Output (200):**

```json
{
  "imageUrl": "/uploads/avatars/..."
}
```

---

# Webhooks

## POST /webhooks/nbgrader/grade

**Auth:** None (uses webhook secret in payload)

**What it does:** Receives nbgrader score updates and marks submissions as `grading_pending`.

**Inputs (JSON):**

```json
{
  "assignmentName": "assignment_1",
  "studentId": "jupyterhub-username",
  "submissionId": "optional-nbgrader-id",
  "score": 80.0,
  "maxScore": 100.0,
  "timestamp": "optional",
  "webhookSecret": "shared-secret"
}
```

**Output (200):**

```json
{
  "success": true,
  "pointsAwarded": 0,
  "message": "Submission received for assignment_1 and marked as grading_pending"
}
```

**Errors:** `401` if webhook secret is configured and does not match.

---

# Admin: Resources (Admin Auth)

All endpoints in this section require a Firebase ID token for a user with role `admin`. Returns `403` if authenticated but not admin, `401` if token is missing or invalid.

## GET /admin/resources

**Auth:** Admin (Bearer token)

**What it does:** Lists resources.

**Inputs (Query):**

- `includeHidden` (optional, boolean)

**Output (200):**

```json
{
  "items": [
    {
      "id": 1,
      "title": "...",
      "provider": "...",
      "coverImage": "...",
      "notionUrl": "...",
      "instructor": { "name": "...", "image": "..." },
      "quote": null,
      "visible": true,
      "createdAt": "...",
      "updatedAt": "..."
    }
  ]
}
```

## POST /admin/resources

**Auth:** Admin (Bearer token)

**What it does:** Creates a resource (multipart upload).

**Inputs (Multipart):**

- `title` (required)
- `provider` (required)
- `notionUrl` (optional)
- `instructorName` (optional)
- `visible` (optional, true/false)
- `coverImage` (optional file)
- `instructorImage` (optional file)

**Output (200):**

```json
{
  "item": {
    "id": 1,
    "title": "...",
    "provider": "...",
    "coverImage": "...",
    "notionUrl": "...",
    "instructor": { "name": "...", "image": "..." },
    "quote": null,
    "visible": true,
    "createdAt": "...",
    "updatedAt": "..."
  }
}
```

## GET /admin/resources/:id

**Auth:** Admin (Bearer token)

**What it does:** Fetches a resource by id.

**Inputs (Path):**

- `id` (integer)

**Output (200):** Same shape as create response (`item`).

## PUT /admin/resources/:id

**Auth:** Admin (Bearer token)

**What it does:** Updates a resource (multipart upload).

**Inputs (Multipart):** Same fields as create. `notionUrl` can be cleared by passing an empty string.

**Output (200):** Same shape as create response (`item`).

## DELETE /admin/resources/:id

**Auth:** Admin (Bearer token)

**What it does:** Deletes a resource.

**Output (200):**

```json
{ "success": true }
```

## PATCH /admin/resources/:id/visibility

**Auth:** Admin (Bearer token)

**What it does:** Toggles resource visibility.

**Inputs (JSON):**

```json
{ "visible": true }
```

**Output (200):** Same shape as create response (`item`).

---

# Admin: Certificates (Admin Auth)

## GET /admin/certificates

**Auth:** Admin (Bearer token)

**Inputs (Query):**

- `includeHidden` (optional, boolean)

**Output (200):**

```json
{
  "items": [
    {
      "id": 1,
      "level": "...",
      "title": "...",
      "courseTitle": "...",
      "coverImage": "...",
      "firstName": "...",
      "secondName": "...",
      "courseraUrl": "...",
      "youtubeUrl": "...",
      "visible": true,
      "createdAt": "...",
      "updatedAt": "..."
    }
  ]
}
```

## POST /admin/certificates

**Auth:** Admin (Bearer token)

**What it does:** Creates a certificate (multipart upload).

**Inputs (Multipart):**

- `level` (required)
- `title` (required)
- `courseTitle` (required)
- `firstName` (required)
- `secondName` (required)
- `courseraUrl` (optional)
- `youtubeUrl` (optional)
- `visible` (optional, true/false)
- `coverImage` (optional file)

**Output (200):**

```json
{
  "item": {
    "id": 1,
    "level": "...",
    "title": "...",
    "courseTitle": "...",
    "coverImage": "...",
    "firstName": "...",
    "secondName": "...",
    "courseraUrl": "...",
    "youtubeUrl": "...",
    "visible": true,
    "createdAt": "...",
    "updatedAt": "..."
  }
}
```

## GET /admin/certificates/:id

**Auth:** Admin (Bearer token)

**Inputs (Path):**

- `id` (integer)

**Output (200):** Same shape as create response (`item`).

## PUT /admin/certificates/:id

**Auth:** Admin (Bearer token)

**What it does:** Updates a certificate (multipart upload).

**Inputs (Multipart):** Same fields as create. `courseraUrl` and `youtubeUrl` can be cleared by sending empty strings.

**Output (200):** Same shape as create response (`item`).

## DELETE /admin/certificates/:id

**Auth:** Admin (Bearer token)

**Output (200):**

```json
{ "success": true }
```

## PATCH /admin/certificates/:id/visibility

**Auth:** Admin (Bearer token)

**Inputs (JSON):**

```json
{ "visible": true }
```

**Output (200):** Same shape as create response (`item`).

---

# Admin: Challenges (Admin Auth)

## GET /admin/challenges

**Auth:** Admin (Bearer token)

**Inputs (Query):**

- `includeHidden` (optional, boolean)

**Output (200):**

```json
{
  "items": [
    {
      "id": 1,
      "title": "...",
      "description": "...",
      "allowedSubmissions": 3,
      "startDate": "...",
      "endDate": "...",
      "visible": true,
      "createdAt": "...",
      "updatedAt": "..."
    }
  ]
}
```

## POST /admin/challenges

**Auth:** Admin (Bearer token)

**Inputs (JSON):**

```json
{
  "title": "Challenge",
  "description": "...",
  "week": 1,
  "challengeUrl": "https://...",
  "allowedSubmissions": 3,
  "startDate": "2026-05-01",
  "endDate": "2026-05-08",
  "visible": true
}
```

**Output (200):** Same shape as list item (`item`).

## GET /admin/challenges/:id

**Auth:** Admin (Bearer token)

**Output (200):** Single `item` in the same shape as list.

## PUT /admin/challenges/:id

**Auth:** Admin (Bearer token)

**Inputs (JSON):** Same as create, all fields optional.

**Output (200):** Single `item` in the same shape as list.

## DELETE /admin/challenges/:id

**Auth:** Admin (Bearer token)

**Output (200):**

```json
{ "success": true }
```

## PATCH /admin/challenges/:id/visibility

**Auth:** Admin (Bearer token)

**Inputs (JSON):**

```json
{ "visible": true }
```

**Output (200):** Single `item` in the same shape as list.

## GET /admin/challenges/:id/notebook

**Auth:** Admin (Bearer token)

**What it does:** Returns the notebook record for a challenge.

**Output (200):**

```json
{
  "item": {
    "id": 1,
    "challengeId": 10,
    "assignmentName": "...",
    "notebookFilename": "...",
    "notebookPath": "...",
    "maxPoints": 100,
    "cpuLimit": 0.5,
    "memoryLimit": "512M",
    "timeLimitMinutes": 60,
    "networkDisabled": true,
    "createdAt": "...",
    "updatedAt": "..."
  }
}
```

---

# Admin: Notebooks (Admin Auth)

## GET /admin/notebooks

**Auth:** Admin (Bearer token)

**What it does:** Lists all challenge notebooks.

**Output (200):**

```json
{
  "items": [
    {
      "id": 1,
      "challengeId": 10,
      "assignmentName": "...",
      "notebookFilename": "...",
      "notebookPath": "...",
      "maxPoints": 100,
      "cpuLimit": 0.5,
      "memoryLimit": "512M",
      "timeLimitMinutes": 60,
      "networkDisabled": true,
      "createdAt": "...",
      "updatedAt": "..."
    }
  ]
}
```

## POST /admin/notebooks

**Auth:** Admin (Bearer token)

**What it does:** Uploads a notebook for a challenge (multipart).

**Inputs (Multipart):**

- `challengeId` (required, integer)
- `assignmentName` (required)
- `maxPoints` (optional, integer)
- `cpuLimit` (optional, number)
- `memoryLimit` (optional, string)
- `timeLimitMinutes` (optional, integer)
- `networkDisabled` (optional, true/false)
- `notebook` (required file)

**Output (200):**

```json
{
  "item": {
    "id": 1,
    "challengeId": 10,
    "assignmentName": "...",
    "notebookFilename": "...",
    "notebookPath": "...",
    "maxPoints": 100,
    "cpuLimit": 0.5,
    "memoryLimit": "512M",
    "timeLimitMinutes": 60,
    "networkDisabled": true,
    "createdAt": "...",
    "updatedAt": "..."
  }
}
```

## PUT /admin/notebooks/:id

**Auth:** Admin (Bearer token)

**What it does:** Updates notebook settings (JSON).

**Inputs (JSON):**

```json
{
  "assignmentName": "New Name",
  "maxPoints": 120,
  "cpuLimit": 1.0,
  "memoryLimit": "1G",
  "timeLimitMinutes": 90,
  "networkDisabled": false
}
```

**Output (200):** Same shape as create response (`item`).

## DELETE /admin/notebooks/:id

**Auth:** Admin (Bearer token)

**Output (200):**

```json
{ "success": true }
```

## GET /admin/notebooks/:id/edit

**Auth:** Admin (Bearer token)

**What it does:** Returns a JupyterHub URL and token for admins to edit grading notebooks.

**Output (200):**

```json
{
  "success": true,
  "jupyterhubUrl": "https://...",
  "token": "...",
  "message": "..."
}
```

## POST /admin/notebooks/:id/sync

**Auth:** Admin (Bearer token)

**What it does:** Syncs a notebook into nbgrader source via grading service.

**Output (200):**

```json
{
  "success": true,
  "message": "Notebook '...' synced to nbgrader. Students will now receive the graded version."
}
```

---

# Admin: Submissions (Admin Auth)

## GET /admin/submissions

**Auth:** Admin (Bearer token)

**What it does:** Lists all submissions with user and challenge context.

**Output (200):**

```json
{
  "items": [
    {
      "id": "uuid",
      "userId": "uuid",
      "userName": "...",
      "userEmail": "...",
      "challengeId": 10,
      "challengeTitle": "...",
      "allowedSubmissions": 3,
      "attemptNumber": 1,
      "attemptsUsed": 1,
      "attemptsRemaining": 2,
      "status": "grading_pending",
      "score": 80.0,
      "maxScore": 100.0,
      "pointsAwarded": 80,
      "pointsCredited": false,
      "startedAt": "...",
      "submittedAt": "...",
      "gradedAt": "..."
    }
  ]
}
```

## GET /admin/submissions/:id/access

**Auth:** Admin (Bearer token)

**What it does:** Returns JupyterHub URLs to view and download a submission.

**Output (200):**

```json
{
  "success": true,
  "viewUrl": "https://...",
  "downloadUrl": "https://...",
  "message": "Submission access URL generated successfully"
}
```

## GET /admin/submissions/:id/file

**Auth:** Admin (Bearer token)

**What it does:** Streams the submitted notebook from the grading service.

**Inputs (Query):**

- `download` (optional, boolean) - when true, sets `Content-Disposition: attachment`

**Output:** File response (content type `application/x-ipynb+json` or as returned by grading service).

## POST /admin/submissions/:id/grade

**Auth:** Admin (Bearer token)

**What it does:** Manually grades a submission and credits points.

**Inputs (JSON):**

```json
{ "score": 85.0 }
```

**Output (200):**

```json
{
  "item": {
    "id": "uuid",
    "userId": "uuid",
    "userName": "...",
    "userEmail": "...",
    "challengeId": 10,
    "challengeTitle": "...",
    "allowedSubmissions": 3,
    "attemptNumber": 1,
    "attemptsUsed": 1,
    "attemptsRemaining": 2,
    "status": "graded",
    "score": 85.0,
    "maxScore": 100.0,
    "pointsAwarded": 85,
    "pointsCredited": true,
    "startedAt": "...",
    "submittedAt": "...",
    "gradedAt": "..."
  }
}
```

**Errors:** `400` if score is not between 0 and 100 or submission not in a gradable status.

---

# Static Files

## GET /uploads/\*

**Auth:** None

**What it does:** Serves static files from the `uploads` directory (avatars, covers, notebooks, etc.).
