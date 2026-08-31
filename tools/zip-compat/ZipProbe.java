import java.io.InputStream;
import java.nio.file.Path;
import java.security.MessageDigest;
import java.util.ArrayList;
import java.util.LinkedHashMap;
import java.util.List;
import java.util.Map;
import java.util.zip.ZipEntry;
import java.util.zip.ZipFile;
import java.util.zip.ZipInputStream;

/** Development-only behavioral probe. Production Entrybound never loads this class. */
public final class ZipProbe {
    private static String hex(byte[] bytes) {
        var output = new StringBuilder();
        for (byte value : bytes) output.append(String.format("%02x", value));
        return output.toString();
    }

    private static String digest(byte[] bytes) throws Exception {
        return hex(MessageDigest.getInstance("SHA-256").digest(bytes));
    }

    private static String quote(String value) {
        return "\"" + value.replace("\\", "\\\\").replace("\"", "\\\"") + "\"";
    }

    private static String item(String name, byte[] bytes) throws Exception {
        return "{\"length\":" + bytes.length + ",\"name\":" + quote(name)
            + ",\"sha256\":" + quote(digest(bytes)) + "}";
    }

    private static void zipFile(Path path) throws Exception {
        var names = new ArrayList<String>();
        var selected = new ArrayList<String>();
        try (var source = new ZipFile(path.toFile())) {
            var entries = source.entries();
            while (entries.hasMoreElements()) names.add(entries.nextElement().getName());
            for (String name : new java.util.LinkedHashSet<>(names)) {
                try {
                    ZipEntry entry = source.getEntry(name);
                    try (InputStream input = source.getInputStream(entry)) {
                        selected.add(item(name, input.readAllBytes()));
                    }
                } catch (Exception error) {
                    selected.add("{\"error\":" + quote(error.getClass().getName()) + ",\"name\":" + quote(name) + "}");
                }
            }
        }
        System.out.println("{\"listing\":[" + names.stream().map(ZipProbe::quote).reduce((a,b)->a+","+b).orElse("")
            + "],\"runtime\":\"zip/java-zipfile@21.0.12.1\",\"selected\":[" + String.join(",", selected) + "]}");
    }

    private static void zipInputStream(Path path) throws Exception {
        var names = new ArrayList<String>();
        Map<String, byte[]> selected = new LinkedHashMap<>();
        String streamError = null;
        try (var input = new ZipInputStream(java.nio.file.Files.newInputStream(path))) {
            try {
                ZipEntry entry;
                while ((entry = input.getNextEntry()) != null) {
                    names.add(entry.getName());
                    selected.put(entry.getName(), input.readAllBytes()); // standardized last-sequential projection
                }
            } catch (Exception error) {
                streamError = error.getClass().getName();
            }
        }
        var items = new ArrayList<String>();
        for (var entry : selected.entrySet()) items.add(item(entry.getKey(), entry.getValue()));
        System.out.println("{\"listing\":[" + names.stream().map(ZipProbe::quote).reduce((a,b)->a+","+b).orElse("")
            + "],\"runtime\":\"zip/java-zipinputstream@21.0.12.1\",\"selected\":[" + String.join(",", items) + "]"
            + (streamError == null ? "" : ",\"stream_error\":" + quote(streamError)) + "}");
    }

    public static void main(String[] args) throws Exception {
        if (args[0].equals("zipfile")) zipFile(Path.of(args[1]));
        else zipInputStream(Path.of(args[1]));
    }
}
