// The link an admin's AI hands out (/api/admin/proposals → url): the proposal opens in the Proposals page's detail
import { redirect } from '@sveltejs/kit';

export const load = ({ params }: { params: { id: string } }) => {
	redirect(307, `/proposals?open=${encodeURIComponent(params.id)}`);
};
