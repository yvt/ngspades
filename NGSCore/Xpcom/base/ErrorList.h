// IWYU pragma: private, include "nsError.h"
/* Helper file for nsError.h, via preprocessor magic */
  /* Standard "it worked" return value */
  ERROR(NS_OK,  0),

  /* ======================================================================= */
  /* Core errors, not part of any modules */
  /* ======================================================================= */
  ERROR(NS_ERROR_BASE,                          0xC1F30000),
  /* Returned when an instance is not initialized */
  ERROR(NS_ERROR_NOT_INITIALIZED,               NS_ERROR_BASE + 1),
  /* Returned when an instance is already initialized */
  ERROR(NS_ERROR_ALREADY_INITIALIZED,           NS_ERROR_BASE + 2),
  /* Returned by a not implemented function */
  ERROR(NS_ERROR_NOT_IMPLEMENTED,               0x80004001),
  /* Returned when a given interface is not supported. */
  ERROR(NS_NOINTERFACE,                         0x80004002),
  ERROR(NS_ERROR_NO_INTERFACE,                  NS_NOINTERFACE),
  /* Returned when a function aborts */
  ERROR(NS_ERROR_ABORT,                         0x80004004),
  /* Returned when a function fails */
  ERROR(NS_ERROR_FAILURE,                       0x80004005),
  /* Returned when an unexpected error occurs */
  ERROR(NS_ERROR_UNEXPECTED,                    0x8000ffff),
  /* Returned when a memory allocation fails */
  ERROR(NS_ERROR_OUT_OF_MEMORY,                 0x8007000e),
  /* Returned when an illegal value is passed */
  ERROR(NS_ERROR_ILLEGAL_VALUE,                 0x80070057),
  ERROR(NS_ERROR_INVALID_ARG,                   NS_ERROR_ILLEGAL_VALUE),
  ERROR(NS_ERROR_INVALID_POINTER,               NS_ERROR_INVALID_ARG),
  ERROR(NS_ERROR_NULL_POINTER,                  NS_ERROR_INVALID_ARG),
  /* Returned when a class doesn't allow aggregation */
  ERROR(NS_ERROR_NO_AGGREGATION,                0x80040110),
  /* Returned when an operation can't complete due to an unavailable resource */
  ERROR(NS_ERROR_NOT_AVAILABLE,                 0x80040111),
  /* Returned when a class is not registered */
  ERROR(NS_ERROR_FACTORY_NOT_REGISTERED,        0x80040154),
  /* Returned when a class cannot be registered, but may be tried again later */
  ERROR(NS_ERROR_FACTORY_REGISTER_AGAIN,        0x80040155),
  /* Returned when a dynamically loaded factory couldn't be found */
  ERROR(NS_ERROR_FACTORY_NOT_LOADED,            0x800401f8),
  /* Returned when a factory doesn't support signatures */
  ERROR(NS_ERROR_FACTORY_NO_SIGNATURE_SUPPORT,  NS_ERROR_BASE + 0x101),
  /* Returned when a factory already is registered */
  ERROR(NS_ERROR_FACTORY_EXISTS,                NS_ERROR_BASE + 0x100),


  /* ======================================================================= */
  /* 1: NS_ERROR_MODULE_XPCOM */
  /* ======================================================================= */
#define MODULE NS_ERROR_MODULE_XPCOM
  /* Result codes used by nsIVariant */
  ERROR(NS_ERROR_CANNOT_CONVERT_DATA,       FAILURE(1)),
  ERROR(NS_ERROR_OBJECT_IS_IMMUTABLE,       FAILURE(2)),
  ERROR(NS_ERROR_LOSS_OF_SIGNIFICANT_DATA,  FAILURE(3)),
  /* Result code used by nsIThreadManager */
  ERROR(NS_ERROR_NOT_SAME_THREAD,           FAILURE(4)),
  /* Various operations are not permitted during XPCOM shutdown and will fail
   * with this exception. */
  ERROR(NS_ERROR_ILLEGAL_DURING_SHUTDOWN,   FAILURE(30)),
  ERROR(NS_ERROR_SERVICE_NOT_AVAILABLE,     FAILURE(22)),

  ERROR(NS_SUCCESS_LOSS_OF_INSIGNIFICANT_DATA,  SUCCESS(1)),
  /* Used by nsCycleCollectionParticipant */
  ERROR(NS_SUCCESS_INTERRUPTED_TRAVERSE,        SUCCESS(2)),
  /* DEPRECATED */
  ERROR(NS_ERROR_SERVICE_NOT_FOUND,             SUCCESS(22)),
  /* DEPRECATED */
  ERROR(NS_ERROR_SERVICE_IN_USE,                SUCCESS(23)),
#undef MODULE


  /* ======================================================================= */
  /* 2: NS_ERROR_MODULE_NGSENGINE */
  /* ======================================================================= */
#define MODULE NS_ERROR_MODULE_NGSENGINE

  /*  (placeholder) */
  ERROR(NGS_ERROR_HOGE,          FAILURE(2)),
#undef MODULE

  /* ======================================================================= */
  /* 51: NS_ERROR_MODULE_GENERAL */
  /* ======================================================================= */
#define MODULE NS_ERROR_MODULE_GENERAL
  /* Error code used internally by the incremental downloader to cancel the
   * network channel when the download is already complete. */
  ERROR(NS_ERROR_DOWNLOAD_COMPLETE,      FAILURE(1)),
  /* Error code used internally by the incremental downloader to cancel the
   * network channel when the response to a range request is 200 instead of
   * 206. */
  ERROR(NS_ERROR_DOWNLOAD_NOT_PARTIAL,   FAILURE(2)),
  ERROR(NS_ERROR_UNORM_MOREOUTPUT,       FAILURE(33)),

  ERROR(NS_ERROR_DOCSHELL_REQUEST_REJECTED,  FAILURE(1001)),
  /* This is needed for displaying an error message when navigation is
   * attempted on a document when printing The value arbitrary as long as it
   * doesn't conflict with any of the other values in the errors in
   * DisplayLoadError */
  ERROR(NS_ERROR_DOCUMENT_IS_PRINTMODE,  FAILURE(2001)),

  ERROR(NS_SUCCESS_DONT_FIXUP,           SUCCESS(1)),
  /* This success code may be returned by nsIAppStartup::Run to indicate that
   * the application should be restarted.  This condition corresponds to the
   * case in which nsIAppStartup::Quit was called with the eRestart flag. */
  ERROR(NS_SUCCESS_RESTART_APP,          SUCCESS(1)),
  ERROR(NS_SUCCESS_RESTART_APP_NOT_SAME_PROFILE,    SUCCESS(3)),
  ERROR(NS_SUCCESS_UNORM_NOTFOUND,  SUCCESS(17)),


  /* a11y */
  /* raised when current pivot's position is needed but it's not in the tree */
  ERROR(NS_ERROR_NOT_IN_TREE,  FAILURE(38)),

  /* see nsTextEquivUtils */
  ERROR(NS_OK_NO_NAME_CLAUSE_HANDLED,  SUCCESS(34))
#undef MODULE
