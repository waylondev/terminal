package dev.waylon.terminal.boundedcontexts.terminalsession.infrastructure.config

import dev.waylon.terminal.boundedcontexts.terminalsession.application.useCase.CreateTerminalSessionUseCase
import dev.waylon.terminal.boundedcontexts.terminalsession.application.useCase.GetAllTerminalSessionsUseCase
import dev.waylon.terminal.boundedcontexts.terminalsession.application.useCase.GetTerminalSessionByIdUseCase
import dev.waylon.terminal.boundedcontexts.terminalsession.application.useCase.ResizeTerminalInput
import dev.waylon.terminal.boundedcontexts.terminalsession.application.useCase.ResizeTerminalUseCase
import dev.waylon.terminal.boundedcontexts.terminalsession.application.useCase.TerminateTerminalSessionUseCase
import dev.waylon.terminal.boundedcontexts.terminalsession.domain.TerminalSize
import dev.waylon.terminal.boundedcontexts.terminalsession.domain.exception.TerminalSessionException
import dev.waylon.terminal.boundedcontexts.terminalsession.domain.exception.TerminalSessionNotFoundException
import dev.waylon.terminal.boundedcontexts.terminalsession.infrastructure.dto.CreateSessionRequest
import dev.waylon.terminal.boundedcontexts.terminalsession.infrastructure.dto.ResizeTerminalRequest
import io.ktor.http.ContentType
import io.ktor.http.HttpStatusCode
import io.ktor.server.application.Application
import io.ktor.server.application.log
import io.ktor.server.request.receive
import io.ktor.server.response.respond
import io.ktor.server.response.respondBytes
import io.ktor.server.routing.delete
import io.ktor.server.routing.get
import io.ktor.server.routing.post
import io.ktor.server.routing.route
import io.ktor.server.routing.routing
import kotlinx.coroutines.flow.toList
import kotlinx.serialization.Serializable
import kotlinx.serialization.SerializationException
import org.koin.ktor.ext.inject

/**
 * Response data classes
 */

@Serializable
data class TerminalResizeResponse(
    val sessionId: String,
    val terminalSize: TerminalSize,
    val status: String
)

@Serializable
data class TerminalInterruptResponse(
    val sessionId: String,
    val status: String
)

@Serializable
data class TerminalTerminateResponse(
    val sessionId: String,
    val reason: String,
    val status: String
)

@Serializable
data class TerminalStatusResponse(
    val status: String
)

/**
 * Terminal session routes configuration
 * This follows the same pattern as other route configurations
 * Route layer only handles HTTP requests and responses, business logic is encapsulated in UseCase
 */
fun Application.configureTerminalSessionRoutes() {
    val log = this.log

    // Inject UseCases instead of directly injecting service layer
    val createTerminalSessionUseCase by inject<CreateTerminalSessionUseCase>()
    val getAllTerminalSessionsUseCase by inject<GetAllTerminalSessionsUseCase>()
    val getTerminalSessionByIdUseCase by inject<GetTerminalSessionByIdUseCase>()
    val resizeTerminalUseCase by inject<ResizeTerminalUseCase>()
    val terminateTerminalSessionUseCase by inject<TerminateTerminalSessionUseCase>()

    routing {
        // API routes with /api prefix
        route("/api") {
            route("/sessions") {
                // Create a new session - suspend function to support coroutines
                post {
                    log.debug("Creating new terminal session")
                    try {
                        // Receive request body
                        val request = call.receive<CreateSessionRequest>()

                        // Execute business logic using UseCase - suspend function
                        val session = createTerminalSessionUseCase(request)

                        call.respond(HttpStatusCode.Created, session)
                    } catch (e: SerializationException) {
                        // Invalid request format
                        log.error("Invalid request format: {}", e.message)
                        call.respond(HttpStatusCode.BadRequest, "Invalid request format")
                    } catch (e: IllegalArgumentException) {
                        // Validation failed
                        log.error("Validation failed: {}", e.message)
                        call.respond(HttpStatusCode.BadRequest, mapOf("error" to e.message))
                    } catch (e: Exception) {
                        // Other exceptions
                        log.error("Error creating session: {}", e.message, e)
                        call.respond(HttpStatusCode.InternalServerError, mapOf("error" to "Failed to create session"))
                    }
                }

                // Get all sessions - suspend function to support Flow collection
                get {
                    log.debug("Getting all terminal sessions")
                    try {
                        // Execute business logic using UseCase - returns Flow<TerminalSession>
                        val sessionsFlow = getAllTerminalSessionsUseCase()
                        // Collect Flow to List for HTTP response
                        val sessions = sessionsFlow.toList()

                        call.respond(HttpStatusCode.OK, sessions)
                    } catch (e: TerminalSessionException) {
                        log.error("Terminal session error: {}", e.message, e)
                        call.respond(HttpStatusCode.NotFound, mapOf("error" to e.message))
                    } catch (e: Exception) {
                        log.error("Error getting sessions: {}", e.message, e)
                        call.respond(HttpStatusCode.InternalServerError, mapOf("error" to "Failed to get sessions"))
                    }
                }

                // Get session by ID - suspend function to support coroutines
                get("/{id}") {
                    val id = call.parameters["id"] ?: return@get call.respond(
                        HttpStatusCode.BadRequest,
                        mapOf("error" to "Invalid session ID")
                    )
                    log.debug("Getting session by ID: {}", id)
                    try {
                        // Execute business logic using UseCase - suspend function
                        val session = getTerminalSessionByIdUseCase(id)

                        call.respond(HttpStatusCode.OK, session)
                    } catch (e: TerminalSessionNotFoundException) {
                        log.error("Session not found: {}", e.message, e)
                        call.respond(HttpStatusCode.NotFound, mapOf("error" to e.message))
                    } catch (e: TerminalSessionException) {
                        log.error("Terminal session error: {}", e.message, e)
                        call.respond(HttpStatusCode.BadRequest, mapOf("error" to e.message))
                    } catch (e: Exception) {
                        log.error("Error getting session: {}", e.message, e)
                        call.respond(HttpStatusCode.InternalServerError, mapOf("error" to "Failed to get session"))
                    }
                }

                // Resize terminal - suspend function to support coroutines
                post("/{id}/resize") {
                    val id = call.parameters["id"] ?: return@post call.respond(
                        HttpStatusCode.BadRequest,
                        "Invalid session ID"
                    )

                    try {
                        // Receive request body
                        val request = call.receive<ResizeTerminalRequest>()

                        log.debug(
                            "Resizing terminal session {} to columns: {}, rows: {}",
                            id,
                            request.columns,
                            request.rows
                        )

                        // Execute business logic using UseCase - suspend function
                        val session = resizeTerminalUseCase(ResizeTerminalInput(id, request))

                        // Use dedicated data class for response, directly using TerminalSize object
                        val response = TerminalResizeResponse(
                            sessionId = session.id,
                            terminalSize = session.terminalSize,
                            status = session.status.toString()
                        )

                        call.respond(HttpStatusCode.OK, response)
                    } catch (e: SerializationException) {
                        // Invalid request format
                        log.error("Invalid request format: {}", e.message)
                        call.respond(HttpStatusCode.BadRequest, mapOf("error" to "Invalid request format"))
                    } catch (e: IllegalArgumentException) {
                        // Validation failed
                        log.error("Validation failed: {}", e.message)
                        call.respond(HttpStatusCode.BadRequest, mapOf("error" to e.message))
                    } catch (e: TerminalSessionNotFoundException) {
                        log.error("Session not found: {}", e.message, e)
                        call.respond(HttpStatusCode.NotFound, mapOf("error" to e.message))
                    } catch (e: TerminalSessionException) {
                        log.error("Terminal session error: {}", e.message, e)
                        call.respond(HttpStatusCode.BadRequest, mapOf("error" to e.message))
                    } catch (e: Exception) {
                        // Other exceptions
                        log.error("Error resizing terminal: {}", e.message, e)
                        call.respond(HttpStatusCode.InternalServerError, mapOf("error" to "Failed to resize terminal"))
                    }
                }

                // Terminate session - suspend function to support coroutines
                delete("/{id}") {
                    val id = call.parameters["id"] ?: return@delete call.respond(
                        HttpStatusCode.BadRequest,
                        mapOf("error" to "Invalid session ID")
                    )
                    log.debug("Terminating terminal session: {}", id)
                    try {
                        // Execute business logic using UseCase - suspend function
                        val session = terminateTerminalSessionUseCase(id)

                        val response = TerminalTerminateResponse(
                            sessionId = session.id,
                            reason = "User terminated",
                            status = session.status.toString()
                        )

                        call.respond(HttpStatusCode.OK, response)
                    } catch (e: TerminalSessionNotFoundException) {
                        log.error("Session not found: {}", e.message, e)
                        call.respond(HttpStatusCode.NotFound, mapOf("error" to e.message))
                    } catch (e: TerminalSessionException) {
                        log.error("Terminal session error: {}", e.message, e)
                        call.respond(HttpStatusCode.BadRequest, mapOf("error" to e.message))
                    } catch (e: Exception) {
                        log.error("Error terminating session: {}", e.message, e)
                        call.respond(
                            HttpStatusCode.InternalServerError,
                            mapOf("error" to "Failed to terminate session")
                        )
                    }
                }

                // Download file from terminal session
                get("/{id}/download") {
                    val id = call.parameters["id"] ?: return@get call.respond(
                        HttpStatusCode.BadRequest,
                        mapOf("error" to "Invalid session ID")
                    )

                    val filePath = call.request.queryParameters["filePath"] ?: return@get call.respond(
                        HttpStatusCode.BadRequest,
                        mapOf("error" to "Missing file path parameter")
                    )

                    log.debug("Downloading file for session {}: {}", id, filePath)
                    try {
                        // Get the session to validate it exists
                        getTerminalSessionByIdUseCase(id)

                        // Create a file object
                        val file = java.io.File(filePath)
                        if (!file.exists() || !file.isFile) {
                            return@get call.respond(
                                HttpStatusCode.NotFound,
                                mapOf("error" to "File not found: $filePath")
                            )
                        }

                        // Read file content
                        val fileBytes = file.readBytes()

                        // Set headers for download - ensure filename is properly quoted
                        call.response.headers.append("Content-Disposition", "attachment; filename=\"${file.name}\"")
                        call.response.headers.append("Content-Type", "application/octet-stream")
                        call.response.headers.append("Content-Length", fileBytes.size.toString())

                        // Send file content
                        call.respondBytes(fileBytes, ContentType.Application.OctetStream, HttpStatusCode.OK)
                    } catch (e: TerminalSessionNotFoundException) {
                        log.error("Session not found: {}", e.message, e)
                        call.respond(HttpStatusCode.NotFound, mapOf("error" to e.message))
                    } catch (e: Exception) {
                        log.error("Error downloading file: {}", e.message, e)
                        call.respond(
                            HttpStatusCode.InternalServerError,
                            mapOf("error" to "Failed to download file")
                        )
                    }
                }
            }
        }
    }
}
