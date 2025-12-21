package dev.waylon.terminal.boundedcontexts.terminalsession.infrastructure.dto

import kotlinx.serialization.Serializable

/**
 * Terminal session creation request body
 * Uses standard JSON request body instead of Query String
 */
@Serializable
data class CreateSessionRequest(
    /**
     * User ID, required field
     */
    val userId: String,
    
    /**
     * Session title, optional field
     */
    val title: String? = null,
    
    /**
     * Working directory, optional field
     */
    val workingDirectory: String? = null,
    
    /**
     * Shell type, optional field
     */
    val shellType: String? = null,
    
    /**
     * Terminal columns, optional field
     */
    val columns: Int? = null,
    
    /**
     * Terminal rows, optional field
     */
    val rows: Int? = null
)

/**
 * Terminal resize request body
 * Uses standard JSON request body instead of Query String
 */
@Serializable
data class ResizeTerminalRequest(
    /**
     * Terminal columns, required field
     */
    val columns: Int,
    
    /**
     * Terminal rows, required field
     */
    val rows: Int
)
