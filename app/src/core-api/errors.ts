// Map core error codes to user-facing Vietnamese messages. Raw technical
// messages from Rust are never shown to the user.
import type { ApiError } from './types';

const CONSTRAINTS: Record<string, string> = {
  WIDTH_LESS_THAN_SIDES: 'Chiều rộng không thể nhỏ hơn tổng chiều dày hai hồi.',
  HEIGHT_TOO_SMALL: 'Chiều cao không đủ để chứa nóc, đáy và chân tủ.',
  DEPTH_TOO_SMALL: 'Chiều sâu quá nhỏ so với tấm hậu và khoảng lùi đợt.',
  THICKNESS_OUT_OF_RANGE: 'Chiều dày ván phải nằm trong khoảng 3–60 mm.',
  TOO_MANY_SHELVES: 'Không đủ chiều cao cho số đợt này.',
  DOOR_TOO_NARROW: 'Cánh tủ sẽ hẹp hơn 50 mm.',
  ZONE_TOO_SMALL: 'Khoang sẽ nhỏ hơn 1 mm hoặc các khoang khóa (LOCK) không đủ chỗ — mở khóa một khoang (AUTO / %) hoặc đổi kích thước khác.',
};

/** Reasons of the shape / merge tools (core keeps English, stable text). */
function SHAPE_REASON(r: string): string {
  const table: [string, string][] = [
    ['too large', 'Kích thước bo/vát lớn hơn cạnh tấm.'],
    ['no matching corner', 'Góc đã chọn không còn là góc của tấm (đã bo/cắt trước đó).'],
    ['do not intersect', 'Hai tấm không giao nhau.'],
    ['removes the whole panel', 'Đường cắt bỏ mất toàn bộ tấm.'],
    ['does not cross', 'Đường cắt không đi qua tấm.'],
    ['two distinct points', 'Đường cắt cần hai điểm khác nhau.'],
    ['different cabinets', 'Các tấm phải thuộc cùng một tủ.'],
    ['different thickness', 'Các tấm phải cùng độ dày.'],
    ['same plane', 'Các tấm phải nằm cùng một mặt phẳng.'],
    ['do not touch', 'Các tấm phải chạm hoặc chồng lên nhau.'],
    ['two or more', 'Chọn từ 2 tấm trở lên (Ctrl+click).'],
    ['cabinet parts', 'Tool này dùng cho tấm thuộc tủ.'],
    ['at least one', 'Chọn ít nhất một tấm.'],
  ];
  return table.find(([k]) => r.includes(k))?.[1] ?? 'Kiểm tra lại tấm đã chọn và thông số.';
}

export interface UserMessage {
  title: string;
  detail: string;
}

export function describeError(e: ApiError, action = 'Không thể thực hiện thao tác.'): UserMessage {
  const d = (e.details ?? {}) as Record<string, unknown>;
  switch (e.code) {
    case 'CONSTRAINT_VIOLATED':
      return {
        title: 'Không thể cập nhật kích thước.',
        detail: CONSTRAINTS[String(d.constraint)] ?? 'Giá trị vi phạm ràng buộc của tủ.',
      };
    case 'DEPENDENCY_CYCLE':
      return { title: 'Công thức không hợp lệ.', detail: 'Công thức tạo ra vòng phụ thuộc (tham số tham chiếu chính nó).' };
    case 'INVALID_PARAMETER':
      if (d.name === 'zone')
        return { title: 'Không thể thêm tấm vào vùng này.', detail: 'Vùng đã được chia theo hướng khác — hãy click ghim một vùng con (ô trống) rồi bấm [TAB].' };
      if (d.name === 'shape' || d.name === 'merge') return { title: 'Không áp được tool.', detail: SHAPE_REASON(String(d.reason ?? '')) };
      if (d.name === 'bay') return { title: 'Không đổi được khoang.', detail: 'Cần ít nhất một khoang AUTO hoặc % để hấp thụ thay đổi.' };
      if (d.name === 'split' && String(d.reason ?? '').includes('smaller')) return { title: 'Không kéo được.', detail: 'Khoang bên cạnh sẽ nhỏ hơn 1 mm.' };
      if (d.name === 'split') return { title: 'Không chia được tấm.', detail: 'Số tấm 2–50, khe 0–100 mm.' };
      if (d.name === 'part') return { title: 'Không áp được tool.', detail: 'Tool này chỉ dùng cho tấm thuộc tủ.' };
      return { title: 'Giá trị không hợp lệ.', detail: 'Kiểm tra lại số hoặc công thức (ví dụ: = cabinet.inner_width - 2).' };
    case 'LOCKED':
      return { title: 'Đối tượng đang bị khóa.', detail: 'Mở khóa đối tượng (hoặc tủ chứa nó) để chỉnh sửa.' };
    case 'NOT_FOUND':
      return { title: action, detail: 'Đối tượng không còn tồn tại.' };
    case 'INVALID_TRANSFORM':
      return { title: 'Không thể di chuyển.', detail: 'Vị trí hoặc góc xoay không hợp lệ.' };
    case 'INVALID_REPARENT':
      return { title: 'Không thể di chuyển trong cây.', detail: 'Chi tiết sinh tự động thuộc về tủ của nó, hoặc đích không chứa được đối tượng.' };
    case 'GEOMETRY_BOOLEAN_FAILED':
      return { title: 'Lỗi hình học.', detail: 'Không thể tạo hình khối cho chi tiết này.' };
    case 'INVALID_FEATURE':
      return { title: 'Gia công không hợp lệ.', detail: 'Vị trí hoặc kích thước gia công nằm ngoài tấm.' };
    case 'UNKNOWN_MATERIAL':
      return { title: 'Vật liệu không tồn tại.', detail: 'Chọn vật liệu từ thư viện.' };
    case 'UNSUPPORTED_VERSION':
      return { title: 'Không mở được dự án.', detail: 'Tệp được tạo bởi phiên bản AIC CAD mới hơn.' };
    case 'INVALID_PROJECT':
      return { title: 'Không mở được dự án.', detail: 'Tệp dự án bị hỏng hoặc không đúng định dạng.' };
    case 'NOTHING_TO':
      return { title: 'Không còn thao tác để hoàn tác/làm lại.', detail: '' };
    default:
      return { title: action, detail: 'Đã xảy ra lỗi trong lõi CAD.' };
  }
}
